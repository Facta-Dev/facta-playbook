//! Typed configuration. Source of truth = `<APP>_`-prefixed env vars.
//! Defaults live here, not scattered around the codebase. Surface mirrors the
//! Go `internal/platform/config/config.go` 1:1: same env names and defaults,
//! Go-style duration strings ("15m", "720h", compound "1h30m") parsed on
//! demand via [`parse_go_duration`].

use std::net::SocketAddr;

use anyhow::{Context, Result};
use serde::Deserialize;

/// C1: known placeholder/template secret values that MUST NOT pass validation,
/// regardless of length. A secret that equals (case-insensitive) or contains
/// any of these is rejected. This catches the .env.example defaults and common
/// "I forgot to change it" values.
const PLACEHOLDER_SECRETS: &[&str] = &[
    "replace-with-strong-random-32-bytes",
    "set-me",
    "set-me-to-32+-random-bytes",
    "changeme",
    "change-me",
    "placeholder",
    "your-secret",
    "<app>_jwt_secret",
];

/// INV-T7 offline gate + 1:1 parity with the Go `config.go` surface.
#[derive(Clone, Deserialize)]
// The six `*_enabled`-family flags mirror the go config env keys 1:1; a flat
// env-mapping struct cannot become an enum without breaking that contract.
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    // Runtime (existing rust shape kept; go <APP>_ENV/_BIND/_LOG_LEVEL)
    /// `development` | `staging` | `production`.
    #[serde(default = "default_env")]
    pub env: String,

    /// Address to bind the HTTP server to.
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,

    /// `tracing-subscriber` env filter (e.g. `info,<app>=debug,sqlx=warn`).
    /// Used as a fallback when `RUST_LOG` is unset.
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// CORS allowlist; comma-separated exact origins. `"*"` matches everything
    /// (development only) — the check is exact otherwise.
    #[serde(default = "default_cors_origin")]
    pub cors_origins: String,

    pub database_url: String,
    #[serde(default = "default_max_conn")]
    pub database_max_connections: u32,
    /// Rust-only knob (go pool has no `MinConns`; the rust pool uses it).
    #[serde(default = "default_min_conn")]
    pub database_min_connections: u32,

    // ── Auth / security (go Auth block, 1:1) ──────────────────────────────
    /// JWT signing secret (>= 32 bytes; placeholder-rejecting validation).
    pub jwt_secret: String,
    /// Argon2 pepper applied on top of the password hash (`<APP>_PASSWORD_PEPPER`).
    /// Optional; empty = disabled. Must be 32+ bytes when set (go parity).
    #[serde(default)]
    pub password_pepper: String,
    /// HMAC secret for signed API keys (>= 32 bytes).
    pub apikey_hmac_secret: String,

    /// `<APP>_ACCESS_TOKEN_TTL` — go-style duration string, default `15m`.
    #[serde(default = "default_access_ttl")]
    pub access_token_ttl: String,
    /// `<APP>_REFRESH_TOKEN_TTL` — default `720h`.
    #[serde(default = "default_refresh_ttl")]
    pub refresh_token_ttl: String,
    /// `<APP>_VERIFY_CODE_TTL` (6-digit email codes) — default `24h`.
    #[serde(default = "default_verify_ttl")]
    pub verify_code_ttl: String,
    /// `<APP>_RESET_CODE_TTL` — default `30m`.
    #[serde(default = "default_reset_ttl")]
    pub reset_code_ttl: String,
    /// `<APP>_AUTH_COOKIE` — when `true`, protected routes also accept the
    /// `<app>_access_token` cookie (bearer always accepted; go parity).
    #[serde(default = "default_false")]
    pub auth_cookie: bool,
    /// `<APP>_RESET_RATE_LIMIT` — max codes per window (default 3).
    #[serde(default = "default_reset_rate_limit")]
    pub reset_rate_limit: i32,
    /// `<APP>_RESET_RATE_WINDOW` — default `5m`.
    #[serde(default = "default_reset_rate_window")]
    pub reset_rate_window: String,
    /// `<APP>_MAX_LOGIN_RETRIES` — failed logins before the retry-lock (default 5).
    #[serde(default = "default_max_login_retries")]
    pub max_login_retries: i32,
    /// `<APP>_LOGOUT_ALL_ON_RESET` — revoke every session when a reset lands.
    #[serde(default = "default_true")]
    pub log_out_all_on_reset: bool,

    // ── OAuth sign-in (go OAuth block, 1:1) ───────────────────────────────
    /// Google One-Tap client id (`<APP>_GOOGLE_CLIENT_ID`). Empty/`None` =
    /// unconfigured — the endpoint answers 401 (go parity).
    #[serde(default)]
    pub google_client_id: Option<String>,
    /// Hard-off switch (`<APP>_GOOGLE_ENABLED`, default `true`).
    #[serde(default = "default_true")]
    pub google_enabled: bool,
    /// GitHub client id / secret (`<APP>_GITHUB_CLIENT_ID`/`<APP>_GITHUB_CLIENT_SECRET`).
    #[serde(default)]
    pub github_client_id: Option<String>,
    #[serde(default)]
    pub github_client_secret: Option<String>,
    /// GitHub ships disabled (default `false`): both github routes answer 403
    /// until enabled with id + secret.
    #[serde(default = "default_false")]
    pub github_enabled: bool,
    /// `<APP>_AUTH_GITHUB_CALLBACK_URL`.
    #[serde(default = "default_github_callback")]
    pub github_callback: String,
    /// `<APP>_AUTH_FRONTEND_SUCCESS` — 302 target after a github login/link.
    #[serde(default = "default_frontend_success")]
    pub frontend_success: String,

    /// `<APP>_ALLOWED_ADMIN_EMAILS`, comma-separated, matched
    /// case-insensitively; empty disables the route entirely (go parity).
    #[serde(default)]
    pub allowed_admin_emails: String,

    // ── SMTP (empty host => console sender; offline forces console) ────────
    /// `<APP>_SMTP_HOST` (empty => console sender; `<APP>_OFFLINE=true` forces console).
    #[serde(default)]
    pub smtp_host: String,
    /// `<APP>_SMTP_PORT` (default 587).
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_user: String,
    #[serde(default)]
    pub smtp_pass: String,
    #[serde(default)]
    pub smtp_from: String,

    /// OTLP endpoint. Empty / `None` disables tracing export (no socket opened).
    #[serde(default)]
    pub otel_exporter_otlp_endpoint: Option<String>,

    #[serde(default = "default_false")]
    pub metrics_enabled: bool,

    /// INV-T7 offline gate: when `true`, `AppState::from_env` selects the
    /// Null-Object `NoopMailer` unconditionally — `mailer_endpoint` is never
    /// read and no `reqwest::Client` (socket) is constructed.
    #[serde(default = "default_false")]
    pub offline: bool,

    /// Mailer provider endpoint. `None` selects `NoopMailer`; `Some(url)`
    /// selects `ProviderMailer`. Ignored entirely when `offline` is `true`.
    #[serde(default)]
    pub mailer_endpoint: Option<String>,

    // ── Email outbox worker + janitor (go worker block, 1:1) ──────────────
    /// `<APP>_OUTBOX_POLL` — outbox drain tick, default `2s`.
    #[serde(default = "default_outbox_poll")]
    pub outbox_poll: String,
    /// `<APP>_OUTBOX_BATCH` — claims per tick, default 10.
    #[serde(default = "default_outbox_size")]
    pub outbox_size: usize,
    /// `<APP>_JANITOR_POLL` — purge tick for expired sessions/codes, default `1h`.
    #[serde(default = "default_janitor_poll")]
    pub janitor_poll: String,
}

impl std::fmt::Debug for Config {
    /// Never print secrets. `jwt_secret`, `password_pepper`, `apikey_hmac_secret`,
    /// `smtp_pass` and `github_client_secret` show as `<redacted>` so a config
    /// dump (panic, trace) can't leak them.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const REDACTED: &str = "<redacted>";
        let redact_opt = |o: &Option<String>| o.as_ref().map(|_| REDACTED);
        f.debug_struct("Config")
            .field("env", &self.env)
            .field("bind", &self.bind)
            .field("log_level", &self.log_level)
            .field("cors_origins", &self.cors_origins)
            .field("database_url", &self.database_url)
            .field("database_max_connections", &self.database_max_connections)
            .field("database_min_connections", &self.database_min_connections)
            .field("jwt_secret", &REDACTED)
            .field("password_pepper", &REDACTED)
            .field("apikey_hmac_secret", &REDACTED)
            .field("access_token_ttl", &self.access_token_ttl)
            .field("refresh_token_ttl", &self.refresh_token_ttl)
            .field("verify_code_ttl", &self.verify_code_ttl)
            .field("reset_code_ttl", &self.reset_code_ttl)
            .field("auth_cookie", &self.auth_cookie)
            .field("reset_rate_limit", &self.reset_rate_limit)
            .field("reset_rate_window", &self.reset_rate_window)
            .field("max_login_retries", &self.max_login_retries)
            .field("log_out_all_on_reset", &self.log_out_all_on_reset)
            .field("google_client_id", &self.google_client_id)
            .field("google_enabled", &self.google_enabled)
            .field("github_client_id", &self.github_client_id)
            .field(
                "github_client_secret",
                &redact_opt(&self.github_client_secret),
            )
            .field("github_enabled", &self.github_enabled)
            .field("github_callback", &self.github_callback)
            .field("frontend_success", &self.frontend_success)
            .field("allowed_admin_emails", &self.allowed_admin_emails)
            .field("smtp_host", &self.smtp_host)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_user", &self.smtp_user)
            .field("smtp_pass", &REDACTED)
            .field("smtp_from", &self.smtp_from)
            .field(
                "otel_exporter_otlp_endpoint",
                &self.otel_exporter_otlp_endpoint,
            )
            .field("metrics_enabled", &self.metrics_enabled)
            .field("offline", &self.offline)
            .field("mailer_endpoint", &self.mailer_endpoint)
            .field("outbox_poll", &self.outbox_poll)
            .field("outbox_size", &self.outbox_size)
            .field("janitor_poll", &self.janitor_poll)
            .finish()
    }
}

fn default_env() -> String {
    "development".into()
}
fn default_bind() -> SocketAddr {
    "0.0.0.0:8080"
        .parse()
        .expect("static parse of default bind address")
}
fn default_log_level() -> String {
    "info".into()
}
fn default_cors_origin() -> String {
    "http://localhost:5173".into()
}
const fn default_max_conn() -> u32 {
    20
}
const fn default_min_conn() -> u32 {
    2
}
const fn default_false() -> bool {
    false
}
const fn default_true() -> bool {
    true
}
fn default_access_ttl() -> String {
    "15m".into()
}
fn default_refresh_ttl() -> String {
    "720h".into()
}
fn default_verify_ttl() -> String {
    "24h".into()
}
fn default_reset_ttl() -> String {
    "30m".into()
}
const fn default_reset_rate_limit() -> i32 {
    3
}
fn default_reset_rate_window() -> String {
    "5m".into()
}
const fn default_max_login_retries() -> i32 {
    5
}
fn default_github_callback() -> String {
    "http://localhost:8080/api/v1/auth/github/callback".into()
}
fn default_frontend_success() -> String {
    "http://localhost:5173/auth/success".into()
}
const fn default_smtp_port() -> u16 {
    587
}
fn default_outbox_poll() -> String {
    "2s".into()
}
const fn default_outbox_size() -> usize {
    10
}
fn default_janitor_poll() -> String {
    "1h".into()
}

/// C2/M3: validate a secret field. Trims first (M3), checks the TRIMMED length
/// is at least 32 (C2), then rejects any blocklisted placeholder value (C2).
/// In production, warns if entropy looks low (a 32-char ASCII string is
/// acceptable; the blocklist is the main guard).
fn validate_secret(secret: &str, field: &str, is_production: bool) -> Result<()> {
    let trimmed = secret.trim();
    if trimmed.len() < 32 {
        anyhow::bail!("{field} must be at least 32 bytes (after trim)");
    }
    let lower = trimmed.to_ascii_lowercase();
    for &placeholder in PLACEHOLDER_SECRETS {
        if lower == placeholder || lower.contains(placeholder) {
            anyhow::bail!(
                "{field} is a known placeholder value ('{placeholder}') — set a real secret"
            );
        }
    }
    if is_production {
        // Low-entropy heuristic: a 32-char string of a single repeated char
        // (e.g. "xxxx...x") passes length but is obviously not a secret. The
        // blocklist is the main guard; this is a belt-and-suspenders warn.
        let distinct = trimmed
            .chars()
            .collect::<std::collections::HashSet<_>>()
            .len();
        if distinct < 8 {
            tracing::warn!(
                "{field}: low entropy ({distinct} distinct chars) in production — \
                 rotate to a strong random secret"
            );
        }
    }
    Ok(())
}

/// Go `time.ParseDuration` parity: "15m" → 900s, "720h", compound "1h30m",
/// fractional "0.5h" are all accepted; unitless numbers ("15") are rejected.
/// Mirrors backend-python `config.parse_go_duration` (the working reference).
///
/// # Errors
/// `Err` on empty input, a unitless numeric component, or an unknown unit.
pub fn parse_go_duration(s: &str) -> Result<std::time::Duration> {
    parse_go_duration_secs(s).map(std::time::Duration::from_secs_f64)
}

/// Seconds-valued variant used by the TTL paths (callers convert: services
/// take `chrono::Duration`, worker loops take `std::time::Duration`).
///
/// # Errors
/// `Err` on empty input, a unitless numeric component, or an unknown unit.
pub fn parse_go_duration_secs(s: &str) -> Result<f64> {
    let text = s.trim();
    if text.is_empty() {
        anyhow::bail!("invalid duration {s:?}: empty");
    }
    let text = text.strip_prefix('+').unwrap_or(text);
    if text.starts_with('-') {
        anyhow::bail!("invalid duration {s:?}: must be >= 0");
    }
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut total = 0.0f64;
    while i < len {
        // Digits (one optional '.') form the component value.
        let start = i;
        while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        if start == i {
            anyhow::bail!("invalid duration {s:?}: unitless number at byte {i}");
        }
        let value: f64 = text[start..i]
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid duration {s:?}: {e}"))?;
        // Letters up to the next digit form the unit.
        let unit_start = i;
        while i < len && !bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mult: f64 = match &text[unit_start..i] {
            "ns" => 0.000_000_001,
            "us" | "µs" | "μs" => 0.000_001,
            "ms" => 0.001,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3_600.0,
            other => anyhow::bail!("invalid duration {s:?}: unknown unit {other:?}"),
        };
        total += value * mult;
    }
    Ok(total)
}

/// `chrono::Duration` variant used by the service constructors (the pinned
/// identity services take `chrono::Duration` TTLs; worker loops take
/// `std::time::Duration` via [`parse_go_duration`] — a negative never occurs
/// because [`parse_go_duration_secs`] rejects the sign, so `from_std` cannot
/// fail in practice and errors are surfaced anyway).
///
/// # Errors
/// `Err` on any [`parse_go_duration_secs`] failure or an out-of-range value.
pub fn parse_go_duration_chrono(s: &str) -> Result<chrono::Duration> {
    parse_go_duration_secs(s)
        .map(std::time::Duration::from_secs_f64)
        .and_then(|d| chrono::Duration::from_std(d).map_err(std::convert::Into::into))
}

impl Config {
    /// Load from process env. Uses the `config` crate so file-based overrides
    /// can be added later without touching call sites. Fails fast on invalid
    /// or missing required values.
    ///
    /// # Errors
    /// Returns `Err` if the config cannot be built, deserialised, or fails
    /// [`Config::validate`].
    pub fn from_env() -> Result<Self> {
        let raw = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .try_parsing(true)
                    .separator("__")
                    .prefix("<APP>")
                    .prefix_separator("_"),
            )
            // Unprefixed OTEL vars (standard OpenTelemetry env var names).
            .set_default(
                "otel_exporter_otlp_endpoint",
                std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            )?
            .build()
            .context("building config")?;

        let cfg: Self = raw.try_deserialize().context("deserialising config")?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Boot-time validation. Fail fast on a configuration that would silently
    /// misbehave or be insecure. Go `config.Load()` parity: JWT secret
    /// strength, optional pepper strength, offline-SMTP conflict, plus the
    /// rust-only env-label check and duration parsing.
    fn validate(&self) -> Result<()> {
        if self.database_url.trim().is_empty() {
            anyhow::bail!("<APP>_DATABASE_URL is required");
        }
        validate_secret(
            &self.jwt_secret,
            "<APP>_JWT_SECRET",
            self.is_production(),
        )?;
        validate_secret(
            &self.apikey_hmac_secret,
            "<APP>_APIKEY_HMAC_SECRET",
            self.is_production(),
        )?;
        // password_pepper is optional (empty = disabled) — go parity: when
        // set, 32+ bytes.
        if !self.password_pepper.trim().is_empty() {
            validate_secret(
                &self.password_pepper,
                "<APP>_PASSWORD_PEPPER",
                self.is_production(),
            )?;
        }
        if !matches!(self.env.as_str(), "development" | "staging" | "production") {
            anyhow::bail!("<APP>_ENV must be one of: development, staging, production");
        }
        // INV-T7: offline forbids SMTP credentials (go parity, verbatim).
        if self.offline
            && (!self.smtp_host.is_empty()
                || !self.smtp_user.is_empty()
                || !self.smtp_pass.is_empty())
        {
            anyhow::bail!(
                "<APP>_OFFLINE=true forbids SMTP credentials (INV-T7); use the console sender"
            );
        }
        // Fail fast on unparsable duration knobs (go env.Parse fails at Load).
        for (name, val) in [
            ("<APP>_ACCESS_TOKEN_TTL", &self.access_token_ttl),
            ("<APP>_REFRESH_TOKEN_TTL", &self.refresh_token_ttl),
            ("<APP>_VERIFY_CODE_TTL", &self.verify_code_ttl),
            ("<APP>_RESET_CODE_TTL", &self.reset_code_ttl),
            ("<APP>_RESET_RATE_WINDOW", &self.reset_rate_window),
            ("<APP>_OUTBOX_POLL", &self.outbox_poll),
            ("<APP>_JANITOR_POLL", &self.janitor_poll),
        ] {
            parse_go_duration_secs(val)
                .map_err(|e| anyhow::anyhow!("{name}: {e}"))
                .and_then(|s| {
                    if s <= 0.0 {
                        anyhow::bail!("{name} must be > 0");
                    }
                    Ok(s)
                })
                .with_context(|| format!("invalid {name}"))?;
        }
        // M1: warn (don't block) if production boots with the in-memory
        // revocation store (see the warden's SessionStore contract).
        if self.is_production() {
            tracing::warn!(
                "<APP>_ENV=production: the default in-memory session store is \
                 NOT durable across restarts and NOT shared across instances. \
                 Wire a persistent SessionStore via JwtCodec::with_store before \
                 multi-instance production deployment."
            );
        }
        Ok(())
    }

    /// Whether the server is running in the `production` env.
    #[must_use]
    pub fn is_production(&self) -> bool {
        self.env == "production"
    }

    /// INV-T7 offline gate. When `true`, the composition root selects
    /// `NoopMailer` unconditionally and the oauth clients refuse before any
    /// socket (go `cfg.Offline` parity).
    #[must_use]
    pub const fn is_offline(&self) -> bool {
        self.offline
    }

    /// Cookie-mode name: `<app>_access_token` when `<APP>_AUTH_COOKIE=true`
    /// (go `bearerToken` parity), otherwise `None` — no cookie fallback.
    #[must_use]
    pub fn auth_cookie_name(&self) -> Option<&'static str> {
        self.auth_cookie.then_some(AUTH_COOKIE_NAME)
    }

    /// Go `CookieSecure: cfg.Env != "dev"` parity — verbatim string compare,
    /// so the go default env (`development`) gets `Secure=true`.
    #[must_use]
    pub fn cookie_secure(&self) -> bool {
        self.env != "dev"
    }

    /// Split `<APP>_ALLOWED_ADMIN_EMAILS` (comma-separated) into the
    /// case-insensitive whitelist; empty entries dropped; empty field → none.
    #[must_use]
    pub fn allowed_admin_email_list(&self) -> Vec<String> {
        split_csv_list(&self.allowed_admin_emails)
    }

    /// A fully-populated config for unit tests, with valid placeholders for
    /// every field. Tests mutate only the knobs they exercise.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn test_default() -> Self {
        Self {
            env: "development".into(),
            bind: "127.0.0.1:0".parse().expect("static parse"),
            log_level: "off".into(),
            cors_origins: "http://localhost:5173".into(),
            database_url: "postgres://x".into(),
            database_max_connections: 1,
            database_min_connections: 1,
            jwt_secret: "x".repeat(32),
            password_pepper: String::new(),
            apikey_hmac_secret: "x".repeat(32),
            access_token_ttl: "15m".into(),
            refresh_token_ttl: "720h".into(),
            verify_code_ttl: "24h".into(),
            reset_code_ttl: "30m".into(),
            auth_cookie: false,
            reset_rate_limit: 3,
            reset_rate_window: "5m".into(),
            max_login_retries: 5,
            log_out_all_on_reset: true,
            google_client_id: None,
            google_enabled: true,
            github_client_id: None,
            github_client_secret: None,
            github_enabled: false,
            github_callback: "http://localhost:8080/api/v1/auth/github/callback".into(),
            frontend_success: "http://localhost:5173/auth/success".into(),
            allowed_admin_emails: String::new(),
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_user: String::new(),
            smtp_pass: String::new(),
            smtp_from: String::new(),
            otel_exporter_otlp_endpoint: None,
            metrics_enabled: false,
            offline: false,
            mailer_endpoint: None,
            outbox_poll: "2s".into(),
            outbox_size: 10,
            janitor_poll: "1h".into(),
        }
    }
}

/// Cookie-mode access-token cookie name (go `"<app>_access_token"` parity;
/// rename-substituted at instantiation time). Always bearer-accepted;
/// the cookie is a fallback read only when `auth_cookie` is on.
pub const AUTH_COOKIE_NAME: &str = "<app>_access_token";

/// Comma-separated env value → trimmed, non-empty list (go `envSeparator:","`).
#[must_use]
pub fn split_csv_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_passes_validation() {
        Config::test_default().validate().expect("default is valid");
    }

    #[test]
    fn short_jwt_secret_fails_boot() {
        let mut cfg = Config::test_default();
        cfg.jwt_secret = "short".into();
        let err = cfg.validate().expect_err("short secret must fail");
        assert!(err.to_string().contains("JWT_SECRET"), "{err}");
    }

    #[test]
    fn short_apikey_secret_fails_boot() {
        let mut cfg = Config::test_default();
        cfg.apikey_hmac_secret = "short".into();
        let err = cfg.validate().expect_err("short secret must fail");
        assert!(err.to_string().contains("APIKEY_HMAC_SECRET"), "{err}");
    }

    #[test]
    fn short_pepper_fails_boot() {
        // Go parity: pepper, when set, must be 32+ bytes.
        let mut cfg = Config::test_default();
        cfg.password_pepper = "short".into();
        let err = cfg.validate().expect_err("short pepper must fail");
        assert!(err.to_string().contains("PASSWORD_PEPPER"), "{err}");
    }

    #[test]
    fn set_pepper_of_32_passes_validation() {
        let mut cfg = Config::test_default();
        cfg.password_pepper = "p".repeat(32);
        cfg.validate().expect("32-byte pepper must pass");
    }

    #[test]
    fn empty_pepper_passes_validation() {
        // Empty = disabled (go parity).
        let mut cfg = Config::test_default();
        cfg.password_pepper = String::new();
        cfg.validate().expect("empty pepper must pass");
    }

    #[test]
    fn offline_with_smtp_credentials_fails_boot() {
        // Go parity (INV-T7), verbatim.
        let mut cfg = Config::test_default();
        cfg.offline = true;
        cfg.smtp_host = "smtp.example.test".into();
        let err = cfg.validate().expect_err("offline + smtp must fail");
        assert!(err.to_string().contains("forbids SMTP"), "{err}");
    }

    #[test]
    fn empty_database_url_fails_boot() {
        let mut cfg = Config::test_default();
        cfg.database_url = String::new();
        cfg.validate().expect_err("empty db url must fail");
    }

    #[test]
    fn invalid_env_label_fails_boot() {
        let mut cfg = Config::test_default();
        cfg.env = "qa".into();
        let err = cfg.validate().expect_err("bad env must fail");
        assert!(err.to_string().contains("ENV"), "{err}");
    }

    #[test]
    fn unparsable_ttl_fails_boot() {
        // Go env.Parse fails at Load on a bad duration; so do we.
        let mut cfg = Config::test_default();
        cfg.access_token_ttl = "15".into(); // unitless
        let err = cfg.validate().expect_err("unitless duration must fail");
        assert!(err.to_string().contains("ACCESS_TOKEN_TTL"), "{err}");
    }

    #[test]
    fn zero_ttl_values_fail_boot() {
        // An interval of 0 would busy-loop the worker / instantly expire codes.
        let mut cfg = Config::test_default();
        cfg.outbox_poll = "0s".into();
        let err = cfg.validate().expect_err("0 outbox poll must fail");
        assert!(err.to_string().contains("OUTBOX_POLL"), "{err}");
    }

    #[test]
    fn debug_never_leaks_secrets() {
        let mut cfg = Config::test_default();
        cfg.jwt_secret = "sk-super-secret-key-here-at-least-32b".into();
        cfg.password_pepper = "pepper-super-secret-32-bytes-long!!".into();
        cfg.smtp_pass = "smtp-password".into();
        let dump = format!("{cfg:?}");
        assert!(!dump.contains("sk-super-secret"), "jwt_secret leaked");
        assert!(!dump.contains("pepper-super-secret"), "pepper leaked");
        assert!(!dump.contains("smtp-password"), "smtp_pass leaked");
        assert!(
            dump.contains("<redacted>"),
            "secrets should show as <redacted>"
        );
    }

    #[test]
    fn placeholder_jwt_secret_rejected_by_blocklist() {
        let mut cfg = Config::test_default();
        cfg.jwt_secret = "replace-with-strong-random-32-bytes".into();
        let err = cfg.validate().expect_err("placeholder must fail");
        assert!(err.to_string().contains("placeholder"), "{err}");
    }

    #[test]
    fn whitespace_padded_short_secret_rejected_after_trim() {
        let mut cfg = Config::test_default();
        cfg.jwt_secret = format!("{}{}", "x".repeat(10), " ".repeat(25));
        let err = cfg.validate().expect_err("padded short secret must fail");
        assert!(err.to_string().contains("32 bytes"), "{err}");
    }

    // ── Go duration parity ─────────────────────────────────────────────────

    #[test]
    fn parse_go_duration_matches_go_semantics() {
        use std::time::Duration;
        assert_eq!(parse_go_duration("15m").unwrap(), Duration::from_secs(900));
        assert_eq!(
            parse_go_duration("720h").unwrap(),
            Duration::from_secs(720 * 3600)
        );
        assert_eq!(parse_go_duration("2s").unwrap(), Duration::from_secs(2));
        assert_eq!(parse_go_duration("1h").unwrap(), Duration::from_secs(3600));
        // Compound + fractional (go allows both).
        assert_eq!(
            parse_go_duration("1h30m").unwrap(),
            Duration::from_secs(5400)
        );
        assert_eq!(
            parse_go_duration("0.5h").unwrap(),
            Duration::from_secs(1800)
        );
        assert_eq!(
            parse_go_duration("500ms").unwrap(),
            Duration::from_millis(500)
        );
        assert_eq!(
            parse_go_duration(" 5m ").unwrap(),
            Duration::from_secs(300),
            "trim"
        );
    }

    #[test]
    fn parse_go_duration_rejects_bad_input() {
        assert!(parse_go_duration("15").is_err(), "unitless number");
        assert!(parse_go_duration("m").is_err(), "no digits");
        assert!(parse_go_duration("").is_err(), "empty");
        assert!(parse_go_duration("3x").is_err(), "unknown unit");
        assert!(parse_go_duration("-5m").is_err(), "negative");
    }

    // ── New-env-knob defaults (go envDefault parity) ───────────────────────

    #[test]
    fn go_default_values_match() {
        let cfg = Config::test_default();
        assert_eq!(cfg.access_token_ttl, "15m");
        assert_eq!(cfg.refresh_token_ttl, "720h");
        assert_eq!(cfg.verify_code_ttl, "24h");
        assert_eq!(cfg.reset_code_ttl, "30m");
        assert!(!cfg.auth_cookie);
        assert_eq!(cfg.reset_rate_limit, 3);
        assert_eq!(cfg.reset_rate_window, "5m");
        assert_eq!(cfg.max_login_retries, 5);
        assert!(cfg.log_out_all_on_reset);
        assert!(cfg.google_enabled);
        assert!(!cfg.github_enabled);
        assert_eq!(
            cfg.github_callback,
            "http://localhost:8080/api/v1/auth/github/callback"
        );
        assert_eq!(cfg.frontend_success, "http://localhost:5173/auth/success");
        assert_eq!(cfg.smtp_port, 587);
        assert_eq!(cfg.outbox_poll, "2s");
        assert_eq!(cfg.outbox_size, 10);
        assert_eq!(cfg.janitor_poll, "1h");
        assert_eq!(cfg.cors_origins, "http://localhost:5173");
    }

    #[test]
    fn auth_cookie_name_follows_the_mode_flag() {
        let mut cfg = Config::test_default();
        assert!(cfg.auth_cookie_name().is_none(), "off by default");
        cfg.auth_cookie = true;
        assert_eq!(cfg.auth_cookie_name(), Some("<app>_access_token"));
    }

    #[test]
    fn cookie_secure_mirrors_go_env_compare() {
        // Go: CookieSecure = cfg.Env != "dev" (string compare, verbatim).
        let mut cfg = Config::test_default();
        assert!(
            cfg.cookie_secure(),
            "development gets Secure=true (go parity)"
        );
        cfg.env = "dev".into();
        assert!(!cfg.cookie_secure(), "exact 'dev' gets Secure=false");
    }

    #[test]
    fn allowed_admin_emails_split_is_case_preserving_and_trimmed() {
        // Matching is case-insensitive downstream; the split preserves input.
        let mut cfg = Config::test_default();
        cfg.allowed_admin_emails = " A@X.test, b@y.test ,,".into();
        assert_eq!(cfg.allowed_admin_email_list(), vec!["A@X.test", "b@y.test"]);
        cfg.allowed_admin_emails = String::new();
        assert!(
            cfg.allowed_admin_email_list().is_empty(),
            "empty = route off"
        );
    }
}
