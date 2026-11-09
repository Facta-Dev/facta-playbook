//! GitHub authorization-code client (go `internal/platform/oauthx/github.go`
//! parity): authorize URL, token exchange, `/user` + `/user/emails` with the
//! reference's exact noreply email-resolution order.
//!
//! Offline-guarded: `offline` refuses before any socket with
//! [`Error::Upstream`]. Unit tests cover the offline refusal, the authorize
//! URL building, and the pure email-resolution logic; the HTTP flow itself
//! is the reqwest seam tests never hit.

use std::time::Duration;

use <app>_core::{Error, Result};
use serde::Deserialize;

/// Production GitHub endpoints (go `oauthx` constants).
const GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";

/// The consent scope (go `githubScope`).
const GITHUB_SCOPE: &str = "user:email";

/// GitHub's placeholder addresses are never trusted when something real
/// exists (go `noreplyHost`).
pub const NOREPLY_HOST: &str = "noreply.github.com";

/// The GitHub identity produced by [`GitHubClient::exchange`] (go
/// `oauthx.GitHubUser`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubUser {
    pub id: String,
    pub login: String,
    pub name: String,
    pub email: String,
    pub avatar_url: String,
}

/// The thin GitHub-side client (go `oauthx.GitHubClient`). Endpoints are
/// private production constants — unit tests cannot reach the network.
#[derive(Debug, Clone)]
pub struct GitHubClient {
    offline: bool,
}

impl GitHubClient {
    /// Build the client; `offline` refuses every network step before any
    /// socket (go `NewGitHubClient`).
    #[must_use]
    pub const fn new(offline: bool) -> Self {
        Self { offline }
    }

    /// Build the consent redirect URL: `client_id`, `redirect_uri`,
    /// `scope=user:email` and the (optional) `state` passthrough — go
    /// `Authorize` parity. No network, always allowed.
    ///
    /// # Panics
    /// Cannot: the URL is a module constant.
    #[must_use]
    pub fn authorize(&self, client_id: &str, redirect_uri: &str, state: &str) -> String {
        let mut url = reqwest::Url::parse(GITHUB_AUTHORIZE_URL).expect("constant url");
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", GITHUB_SCOPE);
        if !state.is_empty() {
            pairs.append_pair("state", state);
        }
        drop(pairs);
        url.to_string()
    }

    /// Turn an authorization code into the GitHub identity: token exchange,
    /// `/user`, then `/user/emails` with the reference's preference order. A
    /// failed `/user/emails` is a warning, never a hard error.
    ///
    /// # Errors
    /// - [`Error::Upstream`] on an offline refusal (`offline mode forbids
    ///   network calls`) or any exchange/profile fetch failure.
    pub async fn exchange(
        &self,
        code: &str,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<GitHubUser> {
        if self.offline {
            return Err(Error::Upstream {
                service: "github",
                detail: "offline mode forbids network calls".into(),
            });
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| upstream(format!("http client: {e}")))?;
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];
        // reqwest 0.13 gates `.form()` behind a feature the workspace does not
        // enable; the body is the same urlencoded shape go's `url.Values`
        // produces (ponytail: form_urlencoded, stdlib-of-the-url-world).
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish();
        let resp = client
            .post(GITHUB_TOKEN_URL)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(reqwest::header::ACCEPT, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| upstream(format!("token exchange: {e}")))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| upstream(format!("token read: {e}")))?;
        if !status.is_success() {
            return Err(upstream(format!("token http {status}: {body}")));
        }
        let token: TokenResponse =
            serde_json::from_str(&body).map_err(|e| upstream(format!("token parse: {e}")))?;
        let Some(access_token) = token.access_token.filter(|t| !t.is_empty()) else {
            if let Some(err) = token.error.filter(|e| !e.is_empty()) {
                return Err(upstream(format!(
                    "github {err} — {}",
                    token.error_description.unwrap_or_default()
                )));
            }
            return Err(upstream("github authorization failed".into()));
        };
        let user: UserResponse = client
            .get(GITHUB_USER_URL)
            .bearer_auth(&access_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| upstream(format!("user fetch: {e}")))?
            .error_for_status()
            .map_err(|e| upstream(format!("user fetch: {e}")))?
            .json()
            .await
            .map_err(|e| upstream(format!("user parse: {e}")))?;
        // A failed /user/emails is a warning, never a hard error: scope-
        // restricted tokens may never see it (go parity).
        let emails: Vec<GhEmail> = match client
            .get(GITHUB_EMAILS_URL)
            .bearer_auth(&access_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
            _ => Vec::new(),
        };
        Ok(to_user(user, &emails))
    }
}

/// Assemble the [`GitHubUser`] from the raw profile + emails with the
/// reference's noreply policy (free fn so offline tests reach it).
fn to_user(user: UserResponse, emails: &[GhEmail]) -> GitHubUser {
    let id = user.id.to_string();
    let mut email = user.email.unwrap_or_default();
    if email.contains(NOREPLY_HOST) {
        email.clear();
    }
    if email.is_empty() {
        email = resolve_email(emails);
    }
    if email.is_empty() {
        email = format!("{id}@users.noreply.github.com");
    }
    let name = user.name.unwrap_or_default();
    GitHubUser {
        id,
        login: user.login,
        name,
        email,
        avatar_url: user.avatar_url.unwrap_or_default(),
    }
}

const fn upstream(detail: String) -> Error {
    Error::Upstream {
        service: "github",
        detail,
    }
}

/// The token-exchange reply (go `oauthx` inline struct).
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

/// The `/user` profile (go `oauthx` inline struct).
#[derive(Debug, Deserialize)]
struct UserResponse {
    id: u64,
    login: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

/// One row of `/user/emails` (go `oauthx.ghEmail`).
#[derive(Debug, Deserialize)]
pub(crate) struct GhEmail {
    #[serde(default)]
    email: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    verified: bool,
}

/// Preference order when the profile left the email empty: noreply
/// placeholders never qualify; primary+verified, then any verified, then the
/// first real address (go `resolveEmail` parity). Empty when nothing real
/// survives — the caller then falls back to the id-based noreply address.
fn resolve_email(emails: &[GhEmail]) -> String {
    let real: Vec<&GhEmail> = emails
        .iter()
        .filter(|e| !e.email.ends_with(NOREPLY_HOST))
        .collect();
    if let Some(e) = real.iter().find(|e| e.primary && e.verified) {
        return e.email.clone();
    }
    if let Some(e) = real.iter().find(|e| e.verified) {
        return e.email.clone();
    }
    real.first().map(|e| e.email.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_bears_the_consent_parameters() {
        let client = GitHubClient::new(false);
        let url = client.authorize("cid", "http://cb", "state-1");
        assert!(
            url.starts_with("https://github.com/login/oauth/authorize?"),
            "{url}"
        );
        assert!(url.contains("client_id=cid"), "{url}");
        assert!(url.contains("scope=user%3Aemail"), "{url}");
        assert!(url.contains("state=state-1"), "{url}");
        assert!(url.contains("redirect_uri=http%3A%2F%2Fcb"), "{url}");
    }

    #[test]
    fn authorize_omits_an_empty_state() {
        let client = GitHubClient::new(false);
        let url = client.authorize("cid", "http://cb", "");
        assert!(!url.contains("state="), "{url}");
    }

    #[tokio::test]
    async fn exchange_refuses_offline_before_any_socket() {
        let client = GitHubClient::new(true);
        let err = client
            .exchange("code", "cid", "sec", "http://cb")
            .await
            .unwrap_err();
        match &err {
            Error::Upstream { service, detail } if *service == "github" => {
                assert_eq!(detail, "offline mode forbids network calls", "{detail}");
            }
            other => panic!("offline refusal must be Upstream(github): {other:?}"),
        }
    }

    // The noreply resolution order, offline (go resolveEmail parity).
    #[test]
    fn resolve_email_prefers_primary_verified() {
        let emails = fixture_emails(&[
            ("1001@users.noreply.github.com", false, true),
            ("ada@ex.com", true, true),
            ("other@ex.com", false, true),
        ]);
        assert_eq!(resolve_email(&emails), "ada@ex.com");
    }

    #[test]
    fn resolve_email_prefers_any_verified_then_first_real() {
        let emails = fixture_emails(&[
            ("1001@users.noreply.github.com", true, true),
            ("a@ex.com", false, false),
            ("b@ex.com", false, true),
        ]);
        assert_eq!(resolve_email(&emails), "b@ex.com");
        let only_unverified = fixture_emails(&[("a@ex.com", false, false)]);
        assert_eq!(resolve_email(&only_unverified), "a@ex.com");
    }

    #[test]
    fn resolve_email_skips_noreply_placeholders() {
        let emails = fixture_emails(&[("1001@users.noreply.github.com", true, true)]);
        assert_eq!(resolve_email(&emails), "");
    }

    #[test]
    fn github_user_assembles_the_noreply_policy() {
        // Profile email is the placeholder → resolved from /user/emails.
        let user = to_user(
            profile(
                1001,
                "ada",
                Some("Ada Lovelace"),
                Some("1001@users.noreply.github.com"),
                Some("av"),
            ),
            &fixture_emails(&[("ada@ex.com", true, true)]),
        );
        assert_eq!(user.email, "ada@ex.com");
        assert_eq!(user.id, "1001");
        assert_eq!(user.name, "Ada Lovelace");
        assert_eq!(user.avatar_url, "av");
        // Real profile email is kept even without /user/emails help.
        let user = to_user(profile(1002, "bob", None, Some("real@ex.com"), None), &[]);
        assert_eq!(user.email, "real@ex.com");
        assert_eq!(user.name, "", "missing name defaults empty");
        // No emails access at all → the id-based noreply fallback.
        let user = to_user(profile(1003, "carol", None, None, None), &[]);
        assert_eq!(user.email, "1003@users.noreply.github.com");
    }

    /// Build a raw `/user` profile for the assembly tests.
    fn profile(
        id: u64,
        login: &str,
        name: Option<&str>,
        email: Option<&str>,
        avatar: Option<&str>,
    ) -> UserResponse {
        UserResponse {
            id,
            login: login.to_owned(),
            name: name.map(str::to_owned),
            email: email.map(str::to_owned),
            avatar_url: avatar.map(str::to_owned),
        }
    }

    /// Build raw `/user/emails` rows for the resolution tests.
    fn fixture_emails(rows: &[(&str, bool, bool)]) -> Vec<GhEmail> {
        rows.iter()
            .map(|(email, primary, verified)| GhEmail {
                email: (*email).to_owned(),
                primary: *primary,
                verified: *verified,
            })
            .collect()
    }
}
