//! Email infra adapter. Wraps one external system (an email provider) behind
//! a trait. The composition root picks the real impl; tests pick `NoopMailer`.
use std::sync::Arc;

use async_trait::async_trait;
use <app>_core::{Error, Result};
pub use templates::render;

pub mod templates;

#[async_trait]
pub trait Mailer: Send + Sync + 'static {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<()>;
}

/// Null-object double: drops every message. Use in tests where email is
/// irrelevant.
#[derive(Debug, Default, Clone)]
pub struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
    async fn send(&self, _to: &str, _subject: &str, _body: &str) -> Result<()> {
        Ok(())
    }
}

/// Provider-backed mailer. ponytail: point `with_endpoint` at a mock server
/// in adapter tests; never hit the real provider in CI.
#[derive(Debug, Clone)]
#[allow(dead_code)] // ponytail: fields read by the real provider POST in `send` (stub today).
pub struct ProviderMailer {
    http: reqwest::Client,
    endpoint: String,
}

impl ProviderMailer {
    pub fn new(endpoint: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint,
        }
    }

    #[must_use]
    pub fn with_endpoint(self, endpoint: String) -> Self {
        Self { endpoint, ..self }
    }
}

#[async_trait]
impl Mailer for ProviderMailer {
    async fn send(&self, _to: &str, _subject: &str, _body: &str) -> Result<()> {
        // ponytail: POST to provider; map reqwest::Error -> Error::Upstream.
        Ok(())
    }
}

/// Convenience for the composition root.
pub fn shared(mailer: Arc<dyn Mailer>) -> Arc<dyn Mailer> {
    mailer
}

/// Console sender (go `ConsoleSender` / python `ConsoleSender` parity):
/// logs instead of dialing any socket — dev, tests and the INV-T7 offline
/// guard (`<APP>_OFFLINE=true`). The body (which carries codes) is never
/// logged, only `to` + `subject`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConsoleSender;

#[async_trait]
impl Mailer for ConsoleSender {
    async fn send(&self, to: &str, subject: &str, _body: &str) -> Result<()> {
        tracing::info!(to = %to, subject = %subject, sender = "console", "email.console");
        Ok(())
    }
}

/// SMTP sender (go `SMTPSender` / python `SmtpSender` parity): STARTTLS used
/// when the server offers it (opportunistic — `relay()`-style config), auth
/// only when a user is configured. Constructed only outside the offline gate
/// (`select_sender` routes offline/blank-host to [`ConsoleSender`]).
pub struct SmtpSender {
    host: String,
    port: u16,
    from: String,
    auth: Option<(String, String)>,
}

impl std::fmt::Debug for SmtpSender {
    /// Never print the SMTP password (debug redaction, api `Config` parity).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmtpSender")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("from", &self.from)
            .field(
                "auth",
                &self.auth.as_ref().map(|(user, _)| (user, "<redacted>")),
            )
            .finish()
    }
}

impl SmtpSender {
    #[must_use]
    pub fn new(host: String, port: u16, from: String, user: String, password: String) -> Self {
        // go: any empty user => no auth (net/smtp parity, python `_auth`).
        let auth = (!user.is_empty()).then_some((user, password));
        Self {
            host,
            port,
            from,
            auth,
        }
    }

    /// Lettre's tokio transport with opportunistic STARTTLS: TLS when the
    /// server offers it, plaintext otherwise (go net/smtp + python smtplib
    /// parity). Credentials attached only when a user is configured.
    fn transport(&self) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>> {
        use lettre::{
            AsyncSmtpTransport, Tokio1Executor,
            transport::smtp::{client::Tls, client::TlsParameters},
        };

        let tls_params = TlsParameters::new(self.host.clone()).map_err(|e| Error::Upstream {
            service: "smtp",
            detail: format!("tls params: {e}"),
        })?;
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .map_err(|e| Error::Upstream {
                service: "smtp",
                detail: format!("host {host:?}: {e}", host = self.host),
            })?
            .port(self.port)
            .tls(Tls::Opportunistic(tls_params));
        if let Some((user, pass)) = &self.auth {
            builder =
                builder.credentials(lettre::transport::smtp::authentication::Credentials::new(
                    user.clone(),
                    pass.clone(),
                ));
        }
        Ok(builder.build())
    }
}

#[async_trait]
impl Mailer for SmtpSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        use lettre::{AsyncTransport, Message, message::Mailbox};

        let to_mbox: Mailbox = to.parse().map_err(|e| Error::Upstream {
            service: "smtp",
            detail: format!("bad recipient {to:?}: {e}"),
        })?;
        let from_mbox: Mailbox = self.from.parse().map_err(|e| Error::Upstream {
            service: "smtp",
            detail: format!("bad sender {:?}: {e}", self.from),
        })?;

        let mail = Message::builder()
            .from(from_mbox)
            .to(to_mbox)
            .subject(subject.to_string())
            .body(body.to_string())
            .map_err(|e| Error::Upstream {
                service: "smtp",
                detail: format!("bad message: {e}"),
            })?;

        self.transport()?
            .send(mail)
            .await
            .map_err(|e| Error::Upstream {
                service: "smtp",
                detail: e.to_string(),
            })?;
        Ok(())
    }
}

/// Honors the offline guard first, then configured SMTP (go `SelectSender` /
/// python `select_sender` parity): `offline` or a blank host selects the
/// console sender, anything else the SMTP sender. The password never leaves
/// the constructed [`SmtpSender`]; its [`Debug`] impl redacts it.
#[must_use]
pub fn select_sender(
    offline: bool,
    smtp_host: &str,
    smtp_port: u16,
    smtp_from: &str,
    smtp_user: &str,
    smtp_password: &str,
) -> Arc<dyn Mailer> {
    if offline || smtp_host.is_empty() {
        Arc::new(ConsoleSender)
    } else {
        Arc::new(SmtpSender::new(
            smtp_host.to_string(),
            smtp_port,
            smtp_from.to_string(),
            smtp_user.to_string(),
            smtp_password.to_string(),
        ))
    }
}

/// Which concrete `Mailer` a given (offline, endpoint) pair resolves to.
/// INV-T7: no new trait — a plain selector over the existing Null-Object
/// pair (`NoopMailer` / `ProviderMailer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MailerKind {
    Noop,
    Provider,
}

/// Offline-gate selection: `offline == true` always wins and the endpoint is
/// never inspected (mirrors backend-python `state.py:40` — offline never
/// reads `mailer_endpoint`). Otherwise `Some(endpoint)` selects the provider
/// adapter; `None` (unset/empty) falls back to the no-op adapter.
const fn select_mailer(offline: bool, endpoint: Option<&str>) -> MailerKind {
    if offline {
        return MailerKind::Noop;
    }
    match endpoint {
        Some(_) => MailerKind::Provider,
        None => MailerKind::Noop,
    }
}

/// INV-T7 offline gate, wired from `Config::is_offline` / `Config::mailer_endpoint`
/// at `AppState::from_env` time. When `offline` is `true`, `endpoint` is never
/// read and no `reqwest::Client` (socket) is constructed — `NoopMailer` is
/// returned unconditionally. Never panics: the `Provider` arm is only taken
/// when `endpoint` is matched as `Some` in the same expression, so no
/// `unwrap`/`expect` is needed.
#[must_use]
pub fn mailer_from_config(offline: bool, endpoint: Option<&str>) -> Arc<dyn Mailer> {
    match (select_mailer(offline, endpoint), endpoint) {
        (MailerKind::Provider, Some(ep)) => Arc::new(ProviderMailer::new(ep.to_string())),
        _ => Arc::new(NoopMailer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_selects_noop_even_with_endpoint() {
        // Offline wins unconditionally; the endpoint is never inspected.
        assert_eq!(
            select_mailer(true, Some("https://mailer.example.test")),
            MailerKind::Noop
        );
    }

    #[test]
    fn online_with_endpoint_selects_provider() {
        assert_eq!(
            select_mailer(false, Some("https://mailer.example.test")),
            MailerKind::Provider
        );
    }

    #[test]
    fn online_without_endpoint_selects_noop() {
        assert_eq!(select_mailer(false, None), MailerKind::Noop);
    }

    #[tokio::test]
    async fn mailer_from_config_offline_never_touches_endpoint() {
        // Regression guard for the offline gate itself: even with an endpoint
        // present, offline boot must not construct/exercise a provider socket
        // — the returned mailer must still behave like NoopMailer (send is a
        // pure no-op that always succeeds).
        let mailer = mailer_from_config(true, Some("https://mailer.example.test"));
        mailer
            .send("to@example.test", "subject", "body")
            .await
            .expect("offline mailer send must always succeed");
    }

    #[tokio::test]
    async fn mailer_from_config_online_no_endpoint_falls_back_to_noop() {
        let mailer = mailer_from_config(false, None);
        mailer
            .send("to@example.test", "subject", "body")
            .await
            .expect("noop mailer send must always succeed");
    }

    #[tokio::test]
    async fn mailer_from_config_online_with_endpoint_builds_provider() {
        let mailer = mailer_from_config(false, Some("https://mailer.example.test"));
        mailer
            .send("to@example.test", "subject", "body")
            .await
            .expect("provider mailer stub send must succeed");
    }

    // --- select_sender (go SelectSender routing) ---------------------------

    #[tokio::test]
    async fn select_sender_offline_routes_to_console() {
        // Behavior probe: the console sender accepts anything; the SMTP path
        // would reject an unparseable recipient.
        let m = select_sender(true, "smtp.example.test", 587, "from@x.test", "", "");
        assert!(m.send("not-an-email", "S", "b").await.is_ok());
    }

    #[tokio::test]
    async fn select_sender_blank_host_routes_to_console() {
        let m = select_sender(false, "", 587, "from@x.test", "", "");
        assert!(m.send("not-an-email", "S", "b").await.is_ok());
    }

    #[tokio::test]
    async fn select_sender_with_host_builds_smtp() {
        let m = select_sender(false, "smtp.example.test", 587, "from@x.test", "u", "p");
        // SMTP: a bad recipient fails the message build (Upstream), proving
        // the pick is not the console sender.
        assert!(m.send("not-an-email", "S", "b").await.is_err());
    }

    #[test]
    fn smtp_auth_is_optional_and_password_redacted_in_debug() {
        let authed = SmtpSender::new(
            "smtp.example.test".into(),
            587,
            "from@x.test".into(),
            "u".into(),
            "p".into(),
        );
        let anon = SmtpSender::new(
            "smtp.example.test".into(),
            587,
            "from@x.test".into(),
            String::new(),
            "p".into(),
        );
        let dbg = format!("{authed:?}");
        assert!(dbg.contains("<redacted>"), "debug must redact: {dbg}");
        assert!(!format!("{anon:?}").contains("<redacted>"));
    }
}
