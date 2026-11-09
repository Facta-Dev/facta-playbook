//! Social logins — go `authoauth` usecase.go parity, adapted to the landed
//! `core`/`ports`/`warden` APIs (google one-tap + github oauth, the same
//! login shape as the email flow, `jti == session id`).
use std::sync::Arc;

use chrono::{Duration, Utc};

#[cfg(any(test, feature = "mint"))]
use crate::session::SessionTokens;
use <app>_core::{Error, Result, Role, User, UserId, UserType};
#[cfg(any(test, feature = "mint"))]
use <app>_core::{Session, SessionId};
use <app>_ports::{SessionRepository, UserRepository};
use <app>_warden::JwtCodec;
use <app>_warden::github::{GitHubClient, GitHubUser, NOREPLY_HOST};
use <app>_warden::google::{GoogleIdentity, GoogleVerifier};
use <app>_warden::link_token::LinkToken;
use <app>_warden::token::new_token;

/// go `LinkStatePrefix`: a callback state starting with this string marks an
/// account-linking callback ("link:<jwt>"); anything else is a plain login.
pub const LINK_STATE_PREFIX: &str = "link:";

/// The social-login use-case (go `authoauth.UseCase`): google one-tap
/// (verify → id-match → email-match/link → provision) and github oauth
/// (authorize URLs, login callback, authenticated link flow).
pub struct OAuthService {
    users: Arc<dyn UserRepository>,
    // Only read by the `mint`-gated methods (mint rides the api composition, default-on: go parity).
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    sessions: Arc<dyn SessionRepository>,
    /// Only used by the `mint`-gated google verify path.
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    google: GoogleVerifier,
    github: GitHubClient,
    states: LinkToken,
    /// Only read by the `mint`-gated methods (mint rides the api composition, default-on: go parity).
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    jwt: JwtCodec,
    google_id: Option<String>,
    google_enabled: bool,
    github_id: Option<String>,
    github_secret: Option<String>,
    github_callback: String,
    github_enabled: bool,
    /// go kept the frontend success redirect on the use-case; here handlers
    /// read it from config — stored for parity, never read.
    #[allow(dead_code)]
    frontend: String,
    offline: bool,
    /// Only read by the `mint`-gated methods (mint rides the api composition, default-on: go parity).
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    access_ttl: Duration,
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    refresh_ttl: Duration,
}
impl std::fmt::Debug for OAuthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthService").finish_non_exhaustive()
    }
}
impl OAuthService {
    /// Wires the service (api `state.rs` contract).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn SessionRepository>,
        google: GoogleVerifier,
        github: GitHubClient,
        states: LinkToken,
        jwt: JwtCodec,
        google_id: Option<String>,
        google_enabled: bool,
        github_id: Option<String>,
        github_secret: Option<String>,
        github_callback: String,
        github_enabled: bool,
        frontend: String,
        offline: bool,
        access_ttl: Duration,
        refresh_ttl: Duration,
    ) -> Self {
        Self {
            users,
            sessions,
            google,
            github,
            states,
            jwt,
            google_id,
            google_enabled,
            github_id,
            github_secret,
            github_callback,
            github_enabled,
            frontend,
            offline,
            access_ttl,
            refresh_ttl,
        }
    }

    /// Google verifies the One-Tap credential and issues the session:
    /// google-id match, else email match (verify + link), else provision.
    ///
    /// # Errors
    /// [`Error::Unauthenticated`] ("google sign-in is not configured",
    /// "invalid google token", "email is linked to another account");
    /// [`Error::Forbidden`] for blocked/inactive; port errors as raised.
    #[cfg(any(test, feature = "mint"))]
    pub async fn google(
        &self,
        credential: &str,
        ip: Option<&str>,
        ua: Option<&str>,
    ) -> Result<SessionTokens> {
        if self.offline
            || !self.google_enabled
            || self.google_id.as_deref().is_none_or(str::is_empty)
        {
            return Err(Error::Unauthenticated(
                "google sign-in is not configured".into(),
            ));
        }
        let Ok(id) = self
            .google
            .verify(credential, self.google_id.as_deref().unwrap_or(""))
            .await
        else {
            return Err(Error::Unauthenticated("invalid google token".into()));
        };
        let user = self.find_or_link_google(&id).await?;
        self.start_session(&user, ip, ua).await
    }

    /// google-id match → email-address match (verify + link) → provision.
    /// Provider emails resolve against the stored lowercase convention.
    ///
    /// # Errors
    /// [`Error::Unauthenticated`] ("email is linked to another account")
    /// when a different Google account owns the address; port errors as
    /// raised.
    pub(crate) async fn find_or_link_google(&self, id: &GoogleIdentity) -> Result<User> {
        let email = id.email.trim().to_lowercase();
        if let Some(user) = self.users.get_user_by_google_id(&id.sub).await? {
            return Ok(user);
        }
        if let Some(user) = self.users.get_user_by_email(&email).await? {
            if user.google_id.as_deref().is_some_and(|g| !g.is_empty()) {
                return Err(Error::Unauthenticated(
                    "email is linked to another account".into(),
                ));
            }
            self.users
                .link_google(user.id, &id.sub, &id.picture)
                .await?;
            let mut linked = user.clone();
            linked.google_id = Some(id.sub.clone());
            linked.is_verified = true;
            if linked
                .profile_picture
                .as_deref()
                .is_some_and(|p| !p.is_empty())
            {
                linked.profile_picture.clone_from(&user.profile_picture);
            } else {
                linked.profile_picture = Some(id.picture.clone());
            }
            return Ok(linked);
        }
        let (first, last) = google_names(id);
        let fresh = new_oauth_user(
            &email,
            &first,
            &last,
            &id.picture,
            Some(id.sub.clone()),
            None,
        )?;
        self.users.create_user(&fresh).await?;
        Ok(fresh)
    }

    /// The authorize redirect for the plain login flow (state passthrough);
    /// any refusal is a config problem.
    ///
    /// # Errors
    /// [`Error::Forbidden`] ("github sign-in is not configured") — the go
    /// githubGate is a 403.
    pub fn github_login_url(&self, state: &str) -> Result<String> {
        self.github_gate()?;
        Ok(self.github.authorize(
            self.github_id.as_deref().unwrap_or(""),
            &self.github_callback,
            state,
        ))
    }

    /// The authorize URL carrying `state=link:<jwt>` so the callback links
    /// instead of logging in (authenticated callers only).
    ///
    /// # Errors
    /// Same gate as `github_login_url`; warden link-token errors as raised.
    pub fn github_connect_url(&self, user_id: UserId) -> Result<String> {
        self.github_gate()?;
        let token = self.states.mint_link(&user_id.0.to_string())?;
        Ok(self.github.authorize(
            self.github_id.as_deref().unwrap_or(""),
            &self.github_callback,
            &format!("{LINK_STATE_PREFIX}{token}"),
        ))
    }

    /// The github login callback: exchange the code, resolve the real
    /// email, then github-id match (identity refresh) → email match (link)
    /// → provision.
    ///
    /// # Errors
    /// The github gate, warden exchange errors ([`Error::Upstream`]), the
    /// `find_or_link_` errors, and the start-session gates.
    #[cfg(any(test, feature = "mint"))]
    pub async fn github_login(
        &self,
        code: &str,
        ip: Option<&str>,
        ua: Option<&str>,
    ) -> Result<SessionTokens> {
        self.github_gate()?;
        let gh = self
            .github
            .exchange(
                code,
                self.github_id.as_deref().unwrap_or(""),
                self.github_secret.as_deref().unwrap_or(""),
                &self.github_callback,
            )
            .await?;
        let user = self.find_or_link_github(&gh).await?;
        self.start_session(&user, ip, ua).await
    }

    /// Attaches the freshly exchanged github id to the user named by the
    /// link-state JWT; the link flow changes nothing else (go `GithubLink`).
    ///
    /// # Errors
    /// The github gate; [`Error::Unauthenticated`] ("invalid or expired
    /// link") on a bad state JWT; warden exchange errors; the apply-link
    /// refusals.
    pub async fn github_link(&self, state_jwt: &str, code: &str) -> Result<()> {
        self.github_gate()?;
        let user_id = self
            .states
            .parse_link(state_jwt)
            .map_err(|_| Error::Unauthenticated("invalid or expired link".into()))?;
        let gh = self
            .github
            .exchange(
                code,
                self.github_id.as_deref().unwrap_or(""),
                self.github_secret.as_deref().unwrap_or(""),
                &self.github_callback,
            )
            .await?;
        self.apply_link(&user_id, &gh).await
    }

    /// Attaches the exchanged github id to the target user (go GithubLink):
    /// another user already linked → 401; missing target → 404; a repeated
    /// link of the id the target already carries falls through as a noop
    /// re-assertion (go falls through to `GithubOnly` the same way).
    ///
    /// # Errors
    /// As documented; port errors as raised.
    pub(crate) async fn apply_link(&self, user_id: &str, gh: &GitHubUser) -> Result<()> {
        if let Some(other) = self.users.get_user_by_github_id(&gh.id).await?
            && other.id.0.to_string() != user_id
        {
            return Err(Error::Unauthenticated(
                "github account is already linked to another user".into(),
            ));
        }
        // The same link on the same user: fall through to the re-assert.
        let Ok(uid) = uuid::Uuid::parse_str(user_id) else {
            return Err(Error::NotFound);
        };
        let Some(target) = self.users.get_user_by_id(UserId(uid)).await? else {
            return Err(Error::NotFound);
        };
        self.users.github_only(target.id, &gh.id).await
    }

    /// github-id match → noreply upgrade → email match (link) → provision
    /// (go `findOrLinkGithub`). Provider emails resolve against the stored
    /// lowercase convention.
    ///
    /// # Errors
    /// Port errors as raised.
    pub(crate) async fn find_or_link_github(&self, gh: &GitHubUser) -> Result<User> {
        let gh_email = gh.email.trim().to_lowercase();
        if let Some(user) = self.users.get_user_by_github_id(&gh.id).await? {
            let email = if is_noreply(&user.email) && !gh_email.is_empty() && !is_noreply(&gh_email)
            {
                gh_email.clone()
            } else {
                user.email.clone()
            };
            self.users
                .save_github_login(user.id, &email, &gh.avatar_url)
                .await?;
            let mut refreshed = user.clone();
            refreshed.email = email;
            refreshed.is_verified = true;
            refreshed.profile_picture = Some(keep_picture(
                user.profile_picture.as_deref(),
                &gh.avatar_url,
            ));
            return Ok(refreshed);
        }
        if let Some(user) = self.users.get_user_by_email(&gh_email).await? {
            self.users
                .link_github(user.id, &gh.id, &gh.avatar_url)
                .await?;
            let mut linked = user.clone();
            linked.github_id = Some(gh.id.clone());
            linked.is_verified = true;
            linked.profile_picture = Some(keep_picture(
                user.profile_picture.as_deref(),
                &gh.avatar_url,
            ));
            return Ok(linked);
        }
        let (first, last) = github_names(gh);
        let fresh = new_oauth_user(
            &gh_email,
            &first,
            &last,
            &gh.avatar_url,
            None,
            Some(gh.id.clone()),
        )?;
        self.users.create_user(&fresh).await?;
        Ok(fresh)
    }

    /// Gates account state, stores the refresh session and mints the access
    /// token (`jti == session id`, exactly like the email login).
    ///
    /// # Errors
    /// [`Error::Forbidden`] ("account is blocked"/"account is inactive");
    /// warden mint/token errors as raised.
    #[cfg(any(test, feature = "mint"))]
    pub(crate) async fn start_session(
        &self,
        user: &User,
        ip: Option<&str>,
        ua: Option<&str>,
    ) -> Result<SessionTokens> {
        use <app>_warden::Identity;

        if user.is_blocked {
            return Err(Error::Forbidden("account is blocked".into()));
        }
        if !user.is_active {
            return Err(Error::Forbidden("account is inactive".into()));
        }
        let refresh = new_token()?;
        let session = Session {
            id: SessionId(uuid::Uuid::now_v7()),
            user_id: user.id,
            refresh_token: refresh.clone(),
            ip_address: ip.map(str::to_owned),
            user_agent: ua.map(str::to_owned),
            is_active: true,
            last_activity: Utc::now(),
            expires_at: Utc::now() + self.refresh_ttl,
            created_at: Utc::now(),
        };
        self.sessions.create_session(&session).await?;
        let id = Identity {
            user_id: user.id.0,
            email: Some(user.email.clone()),
            role: Some(user.role.as_db_str().to_owned()),
            // go Identity carries no user_type — the claim is absent.
            user_type: None,
            jti: session.id.0.to_string(),
            roles: vec![user.role],
        };
        let now = Utc::now();
        let access =
            self.jwt
                .mint_identity(&id, now.timestamp(), (now + self.access_ttl).timestamp())?;
        Ok(SessionTokens {
            access_token: access,
            refresh_token: refresh,
            expires_at: now + self.access_ttl,
        })
    }

    /// The one refusal rule of every github route (go `githubGate`); note
    /// the go status asymmetry: github is a 403, google a 401.
    fn github_gate(&self) -> Result<()> {
        let configured = !self.offline
            && self.github_enabled
            && self.github_id.as_deref().is_some_and(|g| !g.is_empty())
            && self.github_secret.as_deref().is_some_and(|s| !s.is_empty());
        if !configured {
            return Err(Error::Forbidden("github sign-in is not configured".into()));
        }
        Ok(())
    }
}

/// go `newOAuthUser`: auto-verified, COMPANY user type (design default),
/// role USER, language "en" — and an unusable random password (the column
/// is NOT NULL; the value is never handed to anyone).
fn new_oauth_user(
    email: &str,
    first: &str,
    last: &str,
    picture: &str,
    google_id: Option<String>,
    github_id: Option<String>,
) -> Result<User> {
    let password = new_token()?;
    Ok(User {
        id: UserId(uuid::Uuid::now_v7()),
        email: email.to_owned(),
        first_name: first.to_owned(),
        last_name: last.to_owned(),
        role: Role::Viewer,
        user_type: UserType::Company,
        is_active: true,
        is_blocked: false,
        is_verified: true,
        retry_count: 0,
        password,
        language: "en".to_owned(),
        profile_picture: (!picture.is_empty()).then(|| picture.to_owned()),
        google_id,
        github_id,
        last_login: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

/// go `googleNames`: given/family fall back to "User"/"Google" when the
/// profile omits them.
#[must_use]
fn google_names(id: &GoogleIdentity) -> (String, String) {
    let first = if id.given_name.trim().is_empty() {
        "User".to_owned()
    } else {
        id.given_name.trim().to_owned()
    };
    let last = if id.family_name.trim().is_empty() {
        "Google".to_owned()
    } else {
        id.family_name.trim().to_owned()
    };
    (first, last)
}

/// go `githubNames`: display name (login fallback) split on the first
/// space; first falls back to "User", last is always "GitHub" unless the
/// name carried a second word.
#[must_use]
fn github_names(gh: &GitHubUser) -> (String, String) {
    let name = if gh.name.trim().is_empty() {
        gh.login.clone()
    } else {
        gh.name.trim().to_owned()
    };
    let mut parts = name.splitn(2, ' ');
    let first = parts.next().unwrap_or("").trim().to_owned();
    let first = if first.is_empty() {
        "User".to_owned()
    } else {
        first
    };
    let last = match parts.next() {
        Some(second) if !second.trim().is_empty() => second.trim().to_owned(),
        _ => "GitHub".to_owned(),
    };
    (first, last)
}

/// go `isNoreply`: GitHub placeholder addresses are never trusted when
/// something real exists.
#[must_use]
fn is_noreply(email: &str) -> bool {
    email.contains(NOREPLY_HOST)
}

/// Keep the stored picture when it is non-empty; otherwise adopt the
/// provider avatar (go keeps the stored picture when set).
fn keep_picture(current: Option<&str>, incoming: &str) -> String {
    match current {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => incoming.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{Fakes, fixture};
    use <app>_warden::github::GitHubClient;
    use <app>_warden::google::GoogleVerifier;
    use <app>_warden::link_token::LinkToken;

    fn service(f: &Fakes) -> OAuthService {
        OAuthService::new(
            f.users.clone(),
            f.sessions.clone(),
            GoogleVerifier::new("", true),
            GitHubClient::new(false),
            LinkToken::new("test-link-secret-at-least-32-bytes"),
            JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!"),
            Some("google-client-id".to_owned()),
            true,
            Some("github-client-id".to_owned()),
            Some("github-secret".to_owned()),
            "https://app.example.com/api/v1/auth/github/callback".to_owned(),
            true,
            "https://app.example.com/app".to_owned(),
            false,
            Duration::seconds(900),
            Duration::days(30),
        )
    }

    fn disabled(f: &Fakes) -> OAuthService {
        OAuthService::new(
            f.users.clone(),
            f.sessions.clone(),
            GoogleVerifier::new("", false),
            GitHubClient::new(false),
            LinkToken::new("test-link-secret-at-least-32-bytes"),
            JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!"),
            None,
            false,
            None,
            None,
            "https://app.example.com/api/v1/auth/github/callback".to_owned(),
            false,
            String::new(),
            true,
            Duration::seconds(900),
            Duration::days(30),
        )
    }

    fn google_identity(email: &str, sub: &str) -> GoogleIdentity {
        GoogleIdentity {
            sub: sub.to_owned(),
            email: email.to_owned(),
            given_name: "Grace".to_owned(),
            family_name: "Hopper".to_owned(),
            picture: "https://img.example.com/g.png".to_owned(),
        }
    }

    #[tokio::test]
    async fn google_gates_and_refuses_bad_credentials() {
        let f = Fakes::new();
        // Unconfigured: offline / disabled / missing client id read the
        // same 401 refusal.
        let svc = disabled(&f);
        let err = svc.google("cred", None, None).await.unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "google sign-in is not configured"
        ));

        // Configured but a junk credential: the verification refuses
        // (the offline verifier refuses before any socket).
        let svc = service(&f);
        let err = svc.google("not-a-jwt", None, None).await.unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "invalid google token"
        ));
        assert!(f.lock().users.is_empty(), "no provisioning on refusal");
    }

    #[tokio::test]
    async fn find_or_link_google_matches_links_and_provisions() {
        let f = Fakes::new();
        let svc = service(&f);

        // Provision: unknown sub + unknown email → a fresh COMPANY user.
        let fresh = svc
            .find_or_link_google(&google_identity("grace@t.dev", "sub-1"))
            .await
            .expect("provisioned");
        assert_eq!(fresh.user_type, UserType::Company);
        assert_eq!(fresh.role, Role::Viewer);
        assert_eq!(fresh.language, "en");
        assert!(fresh.is_active && fresh.is_verified && !fresh.is_blocked);
        assert_eq!(fresh.google_id.as_deref(), Some("sub-1"));
        let stored = {
            let s = f.lock();
            assert_eq!(s.users.len(), 1);
            let stored = s.users.values().next().expect("u").clone();
            drop(s);
            stored
        };
        assert!(!stored.password.is_empty(), "unusably random, never ''");

        // Email match (no google id yet): link + verify + adopt picture.
        let mut existing = fixture::user(2, "linked@t.dev");
        existing.profile_picture = Some("stored.png".to_owned());
        f.lock().users.insert(existing.id, existing.clone());
        let linked = svc
            .find_or_link_google(&google_identity(" linked@T.dev ", "sub-2"))
            .await
            .expect("linked");
        assert_eq!(linked.google_id.as_deref(), Some("sub-2"));
        assert!(linked.is_verified);
        assert_eq!(
            linked.profile_picture.as_deref(),
            Some("stored.png"),
            "a stored picture is kept"
        );

        // sub match wins BEFORE email (identity refresh, no rewrite).
        let matched = svc
            .find_or_link_google(&google_identity("grace@t.dev", "sub-1"))
            .await
            .expect("matched");
        assert_eq!(matched.id, fresh.id);

        // A different Google account owning the email is a 401.
        let mut other = fixture::user(3, "taken@t.dev");
        other.google_id = Some("sub-other".to_owned());
        f.lock().users.insert(other.id, other);
        let err = svc
            .find_or_link_google(&google_identity("taken@t.dev", "sub-9"))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "email is linked to another account"
        ));
    }

    fn gh_user(id: &str, login: &str, name: &str, email: &str) -> GitHubUser {
        GitHubUser {
            id: id.to_owned(),
            login: login.to_owned(),
            name: name.to_owned(),
            email: email.to_owned(),
            avatar_url: "https://img.example.com/gh.png".to_owned(),
        }
    }

    #[tokio::test]
    async fn github_gate_and_urls() {
        let f = Fakes::new();
        // The disabled service refuses with the go 403 message.
        let svc = disabled(&f);
        let err = svc.github_login_url("x").unwrap_err();
        assert!(matches!(
            err,
            Error::Forbidden(m) if m == "github sign-in is not configured"
        ));
        assert!(svc.github_connect_url(crate::fakes::uid(1)).is_err());

        // Enabled: the authorize URL carries client_id, redirect, scope and
        // the state passthrough; connect adds the link: prefix.
        let svc = service(&f);
        let url = svc.github_login_url("state-123").expect("url");
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("client_id=github-client-id"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("scope=user%3Aemail"));
        let url = svc
            .github_connect_url(crate::fakes::uid(7))
            .expect("connect url");
        assert!(url.contains("state=link%3A"), "prefix urlencoded: {url}");
    }

    #[tokio::test]
    async fn apply_link_scopes_the_link() {
        let f = Fakes::new();
        let svc = service(&f);
        let target = fixture::user(1, "ada@t.dev");
        let target_id = target.id.0.to_string();
        f.lock().users.insert(target.id, target.clone());

        // Another user already owns the github id → 401.
        let mut other = fixture::user(2, "grace@t.dev");
        other.github_id = Some("gh-1".to_owned());
        f.lock().users.insert(other.id, other);
        let err = svc
            .apply_link(&target_id, &gh_user("gh-1", "grace", "", ""))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "github account is already linked to another user"
        ));

        // Missing target → 404.
        let err = svc
            .apply_link(
                &crate::fakes::uid(9).0.to_string(),
                &gh_user("gh-9", "ada", "", ""),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound));

        // Happy: the github id lands on the target (github_only).
        svc.apply_link(&target_id, &gh_user("gh-5", "ada", "", ""))
            .await
            .expect("linked");
        assert!(
            f.lock()
                .users
                .values()
                .any(|u| u.github_id.as_deref() == Some("gh-5"))
        );
    }

    /// (gate mutation, go error message) rows for the blocked/inactive gates.
    type GateCase = (fn(&mut User), &'static str);

    #[tokio::test]
    async fn start_session_gates_and_mints() {
        let f = Fakes::new();
        let svc = service(&f);
        let cases: [GateCase; 2] = [
            (|u: &mut User| u.is_blocked = true, "account is blocked"),
            (|u: &mut User| u.is_active = false, "account is inactive"),
        ];
        for (mutate, expected) in cases {
            let mut u = fixture::user(1, "ada@t.dev");
            mutate(&mut u);
            let err = svc.start_session(&u, None, None).await.unwrap_err();
            assert!(matches!(err, Error::Forbidden(m) if m == expected));
        }
        assert!(f.lock().sessions.is_empty(), "gates wrote nothing");

        // Happy: session row stored + tokens with jti == session id.
        let mut happy = fixture::user(2, "hopper@t.dev");
        happy.role = Role::Admin;
        let tokens = svc
            .start_session(&happy, Some("10.0.0.5"), Some("ua/1"))
            .await
            .expect("session");
        let row = f.lock().sessions.values().next().expect("row").clone();
        assert!(row.is_active);
        assert_eq!(row.refresh_token, tokens.refresh_token);
        let claims = svc.jwt.verify(&tokens.access_token).expect("claims");
        assert_eq!(claims.sub, happy.id.0);
        assert_eq!(claims.jti, row.id.0.to_string());
        assert_eq!(claims.email.as_deref(), Some("hopper@t.dev"));
        assert_eq!(claims.role.as_deref(), Some("ADMIN"));
        assert_eq!(claims.user_type, None);
    }

    #[tokio::test]
    async fn github_link_refuses_a_bad_state_before_any_exchange() {
        let f = Fakes::new();
        let svc = service(&f);
        let err = svc.github_link("garbage-state", "code").await.unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "invalid or expired link"
        ));
        assert!(f.lock().users.is_empty(), "no exchange side effects");
    }

    #[tokio::test]
    async fn name_fallbacks_and_noreply_detection() {
        // google: given/family fall back to User/Google.
        let g = GoogleIdentity {
            sub: "sub".to_owned(),
            email: "g@t.dev".to_owned(),
            given_name: String::new(),
            family_name: "  ".to_owned(),
            picture: String::new(),
        };
        assert_eq!(google_names(&g), ("User".to_owned(), "Google".to_owned()));
        let g = google_identity("g@t.dev", "sub");
        assert_eq!(google_names(&g), ("Grace".to_owned(), "Hopper".to_owned()));

        // github: name wins over login; first space splits; last = GitHub.
        assert_eq!(
            github_names(&gh_user("1", "octocat", "", "e@t.dev")),
            ("octocat".to_owned(), "GitHub".to_owned())
        );
        assert_eq!(
            github_names(&gh_user("1", "octocat", "Ada Lovelace", "e@t.dev")),
            ("Ada".to_owned(), "Lovelace".to_owned())
        );
        assert_eq!(
            github_names(&gh_user("1", "octocat", "Prince", "e@t.dev")),
            ("Prince".to_owned(), "GitHub".to_owned())
        );
        assert!(is_noreply("user@noreply.github.com"));
        assert!(!is_noreply("ada@t.dev"));
        assert_eq!(keep_picture(Some("stored.png"), "in.png"), "stored.png");
        assert_eq!(keep_picture(None, "in.png"), "in.png");
        assert_eq!(
            keep_picture(Some(String::new().as_str()), "in.png"),
            "in.png"
        );
    }
}
