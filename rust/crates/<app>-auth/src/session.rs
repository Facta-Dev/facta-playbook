//! Login / refresh / logout / me / session management — go `authsession`
//! usecase.go parity, adapted to the landed `core`/`ports`/`warden` APIs.
//! Token minting rides the warden's `mint_identity` (`jti == session id`) and
//! is therefore compiled only under `test` or the crate's `mint` feature.
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use <app>_core::{Anomaly, Error, Result, Session, SessionId, User, UserId};
use <app>_ports::{SessionRepository, UpdateMeInput, UserRepository};
#[cfg(any(test, feature = "mint"))]
use <app>_warden::Identity;
#[cfg(test)]
use <app>_warden::hash_password;
#[cfg(any(test, feature = "mint"))]
use <app>_warden::verify_password;
use <app>_warden::{JwtCodec, new_token};

/// go `maxAnomalyWindow` (30 days): the anomalies scan refuses longer (or
/// non-positive) windows.
const MAX_ANOMALY_WINDOW_SECS: i64 = 30 * 24 * 60 * 60;

/// Login request (go `LoginInput`); IP/UA ride as parsed by the handler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Login/refresh payload (go `Result`): the short-lived access token, the
/// opaque refresh token and the access expiry instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Session use-case: login (retry lock + state gates), refresh (in-place
/// rotation), logout, me, session management, admin anomalies.
pub struct SessionService {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionRepository>,
    /// Only read by the `mint`-gated methods (mint rides the api composition, default-on: go parity).
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    jwt: JwtCodec,
    pepper: String,
    #[cfg_attr(not(any(test, feature = "mint")), allow(dead_code))]
    access_ttl: Duration,
    refresh_ttl: Duration,
    max_login_retries: i32,
}

impl std::fmt::Debug for SessionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionService").finish_non_exhaustive()
    }
}

impl SessionService {
    /// Wires the service (api `state.rs` contract).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn SessionRepository>,
        jwt: JwtCodec,
        pepper: &str,
        access_ttl: Duration,
        refresh_ttl: Duration,
        max_login_retries: i32,
    ) -> Self {
        Self {
            users,
            sessions,
            jwt,
            pepper: pepper.to_owned(),
            access_ttl,
            refresh_ttl,
            max_login_retries,
        }
    }

    /// Login: password first (wrong-password errors never reveal account
    /// state), retry lock, social guidance, then the account-state gates, the
    /// fresh session and the `jti == session id` access token.
    ///
    /// # Errors
    /// [`Error::Unauthenticated`]("invalid credentials"|"invalid refresh…) —
    /// see the go error list; [`Error::Forbidden`] for blocked/inactive/
    /// unverified/social-guidance; port errors as raised.
    #[cfg(any(test, feature = "mint"))]
    pub async fn login(&self, input: LoginInput) -> Result<SessionTokens> {
        let email = input.email.trim().to_lowercase();
        let Some(user) = self.users.get_user_by_email(&email).await? else {
            return Err(Error::Unauthenticated("invalid credentials".into()));
        };
        if !verify_password(&user.password, &input.password, &self.pepper) {
            let blocked = self
                .users
                .after_failed_login(user.id, self.max_login_retries)
                .await?;
            if blocked {
                return Err(Error::Forbidden("account is blocked".into()));
            }
            if user.google_id.is_some() || user.github_id.is_some() {
                let guidance = social_guidance(
                    user.google_id.as_deref().is_some_and(|g| !g.is_empty()),
                    user.github_id.as_deref().is_some_and(|g| !g.is_empty()),
                );
                return Err(Error::Forbidden(guidance));
            }
            return Err(Error::Unauthenticated("invalid credentials".into()));
        }
        if user.is_blocked {
            return Err(Error::Forbidden("account is blocked".into()));
        }
        if !user.is_active {
            return Err(Error::Forbidden("account is inactive".into()));
        }
        if !user.is_verified {
            return Err(Error::Forbidden("email not verified".into()));
        }
        self.users.after_successful_login(user.id).await?;
        let session = self.new_session(
            input.ip_address.as_deref(),
            input.user_agent.as_deref(),
            user.id,
        )?;
        self.sessions.create_session(&session).await?;
        self.mint_for(&user, &session)
    }

    /// Refresh re-checks the account state, then rotates the row in place:
    /// id + `expires_at` keep the absolute lifetime fixed at login; the CAS
    /// on the old token rejects a replayed token. The identity survives
    /// rotation unchanged (`jti == session id`).
    ///
    /// # Errors
    /// [`Error::Unauthenticated`]("invalid refresh token") on an unknown/
    /// replayed token; [`Error::Forbidden`] for blocked/inactive/unverified.
    #[cfg(any(test, feature = "mint"))]
    pub async fn refresh(
        &self,
        token: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<SessionTokens> {
        let su = self
            .sessions
            .active_session_by_token(token)
            .await?
            .ok_or_else(|| Error::Unauthenticated("invalid refresh token".into()))?;
        if su.is_blocked {
            self.deactivate(su.session.id).await?;
            return Err(Error::Forbidden("account is blocked".into()));
        }
        if !su.user_active {
            self.deactivate(su.session.id).await?;
            return Err(Error::Forbidden("account is inactive".into()));
        }
        if !su.is_verified {
            self.deactivate(su.session.id).await?;
            return Err(Error::Forbidden("email not verified".into()));
        }
        let new_refresh = new_token()?;
        if !self
            .sessions
            .rotate_session(
                su.session.id,
                &su.session.refresh_token,
                &new_refresh,
                ip_address,
                user_agent,
            )
            .await?
        {
            return Err(Error::Unauthenticated("invalid refresh token".into()));
        }
        let id = Identity {
            user_id: su.session.user_id.0,
            email: Some(su.email.clone()),
            role: Some(su.role.as_db_str().to_owned()),
            user_type: None, // the SessionUser join carries no user_type (go parity)
            jti: su.session.id.0.to_string(),
            roles: vec![su.role],
        };
        let access = self.mint(&id)?;
        Ok(SessionTokens {
            access_token: access,
            refresh_token: new_refresh,
            expires_at: Utc::now() + self.access_ttl,
        })
    }

    /// Logout revokes the given refresh token for this user (idempotent).
    ///
    /// # Errors
    /// Port errors as raised.
    pub async fn logout(&self, user_id: UserId, refresh_token: &str) -> Result<()> {
        self.sessions
            .deactivate_by_token_for_user(user_id, refresh_token)
            .await
    }

    /// Me returns the profile of the authenticated user.
    ///
    /// # Errors
    /// [`Error::NotFound`] when the id is unknown.
    pub async fn me(&self, user_id: UserId) -> Result<User> {
        self.users
            .get_user_by_id(user_id)
            .await?
            .ok_or(Error::NotFound)
    }

    /// Applies the non-nil fields and re-reads the profile; unknown id →
    /// [`Error::NotFound`] (go `ErrUserNotFound` parity).
    ///
    /// # Errors
    /// [`Error::NotFound`]; port errors as raised.
    pub async fn update_me(&self, user_id: UserId, input: UpdateMeInput) -> Result<User> {
        self.users
            .update_me(user_id, &input)
            .await?
            .ok_or(Error::NotFound)
    }

    /// The user's sessions, newest activity first (active-only by default).
    ///
    /// # Errors
    /// Port errors as raised.
    pub async fn sessions(&self, user_id: UserId, active_only: bool) -> Result<Vec<Session>> {
        self.sessions
            .list_sessions_by_user(user_id, active_only)
            .await
    }

    /// The user's most recent active session.
    ///
    /// # Errors
    /// Port errors as raised.
    pub async fn current_session(&self, user_id: UserId) -> Result<Option<Session>> {
        self.sessions.current_session(user_id).await
    }

    /// Claims one active session of the user; `false` when the id is absent
    /// (or not one of the user's active sessions) — `""` never claims.
    ///
    /// # Errors
    /// Port errors as raised.
    pub async fn revoke_current(&self, user_id: UserId, session_id: &str) -> Result<bool> {
        if session_id.is_empty() {
            return Ok(false);
        }
        let Ok(id) = uuid::Uuid::parse_str(session_id) else {
            return Ok(false); // unparseable id: simply not one of the user's
        };
        self.sessions
            .revoke_session_for_user(user_id, SessionId(id))
            .await
    }

    /// Deactivates every active session; returns how many were claimed.
    ///
    /// # Errors
    /// Port errors as raised.
    pub async fn revoke_all(&self, user_id: UserId) -> Result<i64> {
        self.sessions.revoke_all_for_user(user_id).await
    }

    /// Admin session report (go Anomalies): expired-but-active rows first,
    /// then users whose active sessions within the window span more than
    /// 3 distinct IPs.
    ///
    /// # Errors
    /// [`Error::Invalid`]("window out of range") outside `(0, 30d]`; port
    /// errors as raised.
    pub async fn anomalies(&self, window: Duration) -> Result<Vec<Anomaly>> {
        if window <= Duration::zero() || window > Duration::seconds(MAX_ANOMALY_WINDOW_SECS) {
            return Err(Error::Invalid("window out of range".into()));
        }
        let expired = self.sessions.count_expired_active().await?;
        let mut out = Vec::new();
        if expired > 0 {
            out.push(Anomaly {
                kind: "expired_but_active".to_owned(),
                count: expired,
                details: serde_json::Value::Null,
            });
        }
        let rows = self.sessions.multi_ip_sessions(Utc::now() - window).await?;
        if !rows.is_empty() {
            out.push(Anomaly {
                kind: "multi_ip_sessions".to_owned(),
                count: i64::try_from(rows.len()).unwrap_or(i64::MAX),
                details: json!({ "user_ids":
                    rows.iter().map(|r| r.user_id.0.to_string()).collect::<Vec<_>>() }),
            });
        }
        Ok(out)
    }

    async fn deactivate(&self, session_id: SessionId) -> Result<()> {
        let _ = self.sessions.deactivate_session(session_id).await?;
        Ok(())
    }

    /// Fresh active refresh session (go `newSession`).
    fn new_session(
        &self,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        user_id: UserId,
    ) -> Result<Session> {
        Ok(Session {
            id: SessionId(uuid::Uuid::now_v7()),
            user_id,
            refresh_token: new_token()?,
            ip_address: ip_address.map(str::to_owned),
            user_agent: user_agent.map(str::to_owned),
            is_active: true,
            last_activity: Utc::now(),
            expires_at: Utc::now() + self.refresh_ttl,
            created_at: Utc::now(),
        })
    }

    /// Issues the access token for a full row (login path: email, role and
    /// `user_type` all present — go `mintFor` parity).
    #[cfg(any(test, feature = "mint"))]
    fn mint_for(&self, user: &User, session: &Session) -> Result<SessionTokens> {
        let id = Identity {
            user_id: user.id.0,
            email: Some(user.email.clone()),
            role: Some(user.role.as_db_str().to_owned()),
            // go Identity carries no user_type — the claim is absent.
            user_type: None,
            jti: session.id.0.to_string(),
            roles: vec![user.role],
        };
        let access = self.mint(&id)?;
        Ok(SessionTokens {
            access_token: access,
            refresh_token: session.refresh_token.clone(),
            expires_at: Utc::now() + self.access_ttl,
        })
    }

    /// The shared mint call (ttl ceiling enforced by the warden).
    #[cfg(any(test, feature = "mint"))]
    fn mint(&self, id: &<app>_warden::Identity) -> Result<String> {
        let now = Utc::now();
        self.jwt
            .mint_identity(id, now.timestamp(), (now + self.access_ttl).timestamp())
    }
}

/// go `socialGuidance`: the wrong-password guidance for an account whose
/// identity came from Google/GitHub (reference message parity).
#[must_use]
fn social_guidance(google: bool, github: bool) -> String {
    match (google, github) {
        (true, true) => concat!(
            "This account was created with Google and GitHub. ",
            "Use one of those to sign in, or reset your password."
        )
        .to_owned(),
        (true, false) => concat!(
            "This account was created with Google. ",
            "Use Google to sign in, or reset your password to set a password."
        )
        .to_owned(),
        _ => concat!(
            "This account was created with GitHub. ",
            "Use GitHub to sign in, or reset your password to set a password."
        )
        .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{Fakes, fixture};

    const PEPPER: &str = "test-pepper";

    /// (gate mutation, go error message) rows for the state gates.
    type GateCase = (fn(&mut User), &'static str);

    fn service(f: &Fakes) -> SessionService {
        SessionService::new(
            f.users.clone(),
            f.sessions.clone(),
            JwtCodec::from_single(b"test-secret-key-at-least-32-bytes!!"),
            PEPPER,
            Duration::seconds(900),
            Duration::days(30),
            3,
        )
    }

    fn login_input(email: &str) -> LoginInput {
        LoginInput {
            email: email.to_owned(),
            password: "password123".to_owned(),
            ip_address: None,
            user_agent: None,
        }
    }

    /// Seed a verified, active user with a real argon2 password hash.
    fn seed_verified(f: &Fakes, email: &str) -> User {
        let mut u = fixture::user(1, email);
        u.is_verified = true;
        u.password = hash_password("password123", PEPPER).expect("hash");
        let id = u.id;
        f.lock().users.insert(id, u.clone());
        u
    }

    #[tokio::test]
    async fn login_happy_path_mints_jti_session() {
        let f = Fakes::new();
        let svc = service(&f);
        let u = seed_verified(&f, "ada@t.dev");
        let id = u.id;
        f.lock().users.get_mut(&id).expect("u").retry_count = 1;

        let t = svc.login(login_input("ada@t.dev")).await.expect("login");
        let s = f.lock();
        let row = s.sessions.values().next().expect("session row").clone();
        assert_eq!(row.user_id, id);
        assert!(row.is_active);
        // jti == session id (deactivating the session revokes the token).
        let claims = svc.jwt.verify(&t.access_token).expect("claims");
        assert_eq!(claims.jti, row.id.0.to_string());
        assert_eq!(claims.sub, id.0);
        assert_eq!(claims.email.as_deref(), Some("ada@t.dev"));
        assert_eq!(claims.user_type, None, "go Identity carries no user_type");
        assert_eq!(
            t.refresh_token, row.refresh_token,
            "login returns the stored refresh token"
        );
        drop(s);
        // after_successful_login reset the retry counter.
        assert_eq!(f.lock().users.get(&id).expect("u").retry_count, 0);
    }

    #[tokio::test]
    async fn login_wrong_password_counts_then_locks() {
        let f = Fakes::new();
        let svc = service(&f);
        seed_verified(&f, "ada@t.dev");

        // Wrong password: attempts increment, error stays 401 (anti-probe).
        for attempt in 1..=2 {
            let err = svc
                .login(LoginInput {
                    password: "wrong-password".to_owned(),
                    ..login_input("ada@t.dev")
                })
                .await
                .unwrap_err();
            assert!(
                matches!(err, Error::Unauthenticated(m) if m == "invalid credentials"),
                "attempt {attempt}"
            );
        }

        // Third failure crosses max_login_retries = 3 → blocked (403).
        let err = svc
            .login(LoginInput {
                password: "wrong-password".to_owned(),
                ..login_input("ada@t.dev")
            })
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Forbidden(m) if m == "account is blocked"));
        assert!(f.lock().users.values().next().expect("u").is_blocked);

        // The right password now trips the is_blocked state gate (still 403).
        let err = svc.login(login_input("ada@t.dev")).await.unwrap_err();
        assert!(matches!(err, Error::Forbidden(m) if m == "account is blocked"));
    }

    #[tokio::test]
    async fn login_social_guidance_and_state_gates() {
        let f = Fakes::new();
        let svc = service(&f);
        // Social accounts: wrong password → the provider guidance (403).
        let google_user = seed_verified(&f, "ada@t.dev");
        f.lock()
            .users
            .get_mut(&google_user.id)
            .expect("u")
            .google_id = Some("g-1".to_owned());
        let err = svc
            .login(LoginInput {
                password: "wrong-password".to_owned(),
                ..login_input("ada@t.dev")
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Forbidden(ref m) if m.contains("Google") && m.contains("reset your password")),
            "guidance: {err:?}"
        );

        // Blocked, inactive and unverified users with the RIGHT password hit
        // the state gates in order (password check first, go parity).
        let cases: [GateCase; 3] = [
            (|u: &mut User| u.is_blocked = true, "account is blocked"),
            (|u: &mut User| u.is_active = false, "account is inactive"),
            (|u: &mut User| u.is_verified = false, "email not verified"),
        ];
        for (mutate, expected) in cases {
            let f = Fakes::new();
            let svc = service(&f);
            let u = seed_verified(&f, "ada@t.dev");
            {
                let mut locked = f.lock();
                mutate(locked.users.get_mut(&u.id).expect("u"));
            }
            let err = svc.login(login_input("ada@t.dev")).await.unwrap_err();
            assert!(
                matches!(err, Error::Forbidden(ref m) if m == expected),
                "gate: {err:?}"
            );
        }
    }

    #[tokio::test]
    async fn refresh_rotates_in_place_and_rejects_replay() {
        let f = Fakes::new();
        let svc = service(&f);
        seed_verified(&f, "ada@t.dev");
        let first = svc.login(login_input("ada@t.dev")).await.expect("login");

        let (row_id, expiry_before) = {
            let row = f.lock().sessions.values().next().expect("row").clone();
            (row.id, row.expires_at)
        };

        let second = svc
            .refresh(&first.refresh_token, Some("10.0.0.2"), Some("ua/2"))
            .await
            .expect("refresh");
        assert_ne!(second.refresh_token, first.refresh_token);
        assert_ne!(second.refresh_token, second.access_token);

        // In-place: the row keeps id + absolute expires_at (CAS kept it).
        let s = f.lock();
        let row = s.sessions.get(&row_id).expect("kept id");
        assert!(row.is_active);
        assert_eq!(row.refresh_token, second.refresh_token);
        assert_eq!(row.ip_address.as_deref(), Some("10.0.0.2"));
        assert!(
            (row.expires_at - expiry_before).num_seconds().abs() < 2,
            "expires_at preserved"
        );
        drop(s);
    }

    #[tokio::test]
    async fn refresh_replay_rejected_and_identity_survives() {
        let f = Fakes::new();
        let svc = service(&f);
        seed_verified(&f, "ada@t.dev");
        let first = svc.login(login_input("ada@t.dev")).await.expect("login");
        let second = svc
            .refresh(&first.refresh_token, None, None)
            .await
            .expect("refresh");

        // Identity survives rotation: same sub + jti (go parity).
        let claims_a = svc.jwt.verify(&first.access_token).expect("claims");
        let claims_b = svc.jwt.verify(&second.access_token).expect("claims");
        assert_eq!(claims_a.jti, claims_b.jti);
        assert_eq!(claims_a.sub, claims_b.sub);
        assert_eq!(claims_b.email.as_deref(), Some("ada@t.dev"));
        assert_eq!(claims_b.role.as_deref(), Some("USER"));

        // Replaying the OLD token must fail the CAS.
        let err = svc
            .refresh(&first.refresh_token, None, None)
            .await
            .unwrap_err();
        assert_eq!(err.status_code(), 401);
        assert!(matches!(err, Error::Unauthenticated(m) if m == "invalid refresh token"));
    }

    #[tokio::test]
    async fn refresh_gates_and_deactivates_bad_accounts() {
        let cases: [GateCase; 3] = [
            (|u: &mut User| u.is_blocked = true, "account is blocked"),
            (|u: &mut User| u.is_active = false, "account is inactive"),
            (|u: &mut User| u.is_verified = false, "email not verified"),
        ];
        for (mutate, expected) in cases {
            let f = Fakes::new();
            let svc = service(&f);
            seed_verified(&f, "ada@t.dev");
            let first = svc.login(login_input("ada@t.dev")).await.expect("login");
            {
                let mut locked = f.lock();
                mutate(locked.users.values_mut().next().expect("u"));
            }

            let err = svc
                .refresh(&first.refresh_token, None, None)
                .await
                .unwrap_err();
            assert!(
                matches!(err, Error::Forbidden(ref m) if m == expected),
                "gate: {err:?}"
            );

            // A refused refresh deactivates the session (go parity).
            let s = f.lock();
            let row = s.sessions.values().next().expect("row").clone();
            drop(s);
            assert!(!row.is_active);
        }
    }

    #[tokio::test]
    async fn logout_me_update_me_and_sessions_listing() {
        let f = Fakes::new();
        let svc = service(&f);
        seed_verified(&f, "ada@t.dev");
        let u = f.lock().users.values().next().expect("u").clone();
        // me: unknown → NotFound; known → the row.
        assert!(matches!(
            svc.me(crate::fakes::uid(9)).await.unwrap_err(),
            Error::NotFound
        ));
        let me = svc.me(u.id).await.expect("me");
        assert_eq!(me.email, u.email);

        // update_me applies and returns the re-read row.
        let updated = svc
            .update_me(
                u.id,
                UpdateMeInput {
                    first_name: Some("Grace".to_owned()),
                    last_name: None,
                    phone: None,
                    picture: None,
                    language: None,
                    address: None,
                    postal: None,
                },
            )
            .await
            .expect("update");
        assert_eq!(updated.first_name, "Grace");

        // A session for ada, so logout has a row to claim.
        let _ada_tokens = svc.login(login_input("ada@t.dev")).await.expect("login");
        // logout with a token that belongs to nobody is a noop; the real
        // token deactivates this user's row (scoped by user too).
        svc.logout(u.id, "ghost-token").await.expect("noop");
        let stored = f.lock().sessions.values().next().expect("row").clone();
        assert!(stored.is_active);
        svc.logout(u.id, &stored.refresh_token)
            .await
            .expect("logout");
        assert!(!f.lock().sessions.get(&stored.id).expect("row").is_active);

        // A second user's logout cannot touch ada's (logged-out) listing.
        let mut u2 = fixture::user(2, "grace@t.dev");
        u2.is_verified = true;
        u2.password = hash_password("password123", PEPPER).expect("hash");
        f.lock().users.insert(u2.id, u2.clone());
        let _other = svc
            .login(LoginInput {
                email: "grace@t.dev".to_owned(),
                ..login_input("ada@t.dev")
            })
            .await
            .expect("second user login");
        // u2's logout against ada's token is a scoped noop (user filter).
        svc.logout(u2.id, &stored.refresh_token)
            .await
            .expect("noop");
        let u2_row = f
            .lock()
            .sessions
            .values()
            .find(|s| s.user_id == u2.id)
            .expect("u2 row")
            .clone();
        assert!(u2_row.is_active);
        // Ada's listing: active_only hides the logged-out row.
        let all = svc.sessions(u.id, false).await.expect("list");
        assert_eq!(all.len(), 1);
        let active = svc.sessions(u.id, true).await.expect("list active");
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn revokes_claim_their_scope() {
        let f = Fakes::new();
        let svc = service(&f);
        let u = seed_verified(&f, "ada@t.dev");

        // "" and unparseable ids claim nothing; the real id claims the row.
        assert!(!svc.revoke_current(u.id, "").await.expect("empty"));
        assert!(
            !svc.revoke_current(u.id, "not-a-uuid")
                .await
                .expect("junk id")
        );
        let first = svc.login(login_input("ada@t.dev")).await.expect("login");
        let row = f.lock().sessions.values().next().expect("row").clone();
        assert!(
            svc.revoke_current(u.id, &row.id.0.to_string())
                .await
                .expect("claim")
        );
        assert!(!f.lock().sessions.get(&row.id).expect("row").is_active);
        let _ = first;

        // revoke_all: two fresh logins, both claimed in one call.
        let _ = svc.login(login_input("ada@t.dev")).await.expect("one");
        let _ = svc.login(login_input("ada@t.dev")).await.expect("two");
        let claimed = svc.revoke_all(u.id).await.expect("all");
        assert_eq!(claimed, 2);
        assert!(svc.sessions(u.id, true).await.expect("list").is_empty());
    }

    #[tokio::test]
    async fn anomalies_report_expired_and_multi_ip() {
        let f = Fakes::new();
        let svc = service(&f);
        let u = seed_verified(&f, "ada@t.dev");
        let _ = svc.login(login_input("ada@t.dev")).await.expect("login");
        // Four more active sessions for the same user, all on distinct IPs.
        for i in 0..4 {
            let mut row = crate::fakes::fixture::session(i + 2, 1, &format!("r-{i}"));
            row.ip_address = Some(format!("10.0.1.{i}"));
            f.lock().sessions.insert(row.id, row);
        }

        // Window validation first.
        let err = svc.anomalies(Duration::zero()).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "window out of range"));
        let err = svc
            .anomalies(Duration::seconds(MAX_ANOMALY_WINDOW_SECS + 1))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "window out of range"));

        // 5 sessions across 5 IPs for one user → multi_ip_sessions.
        let out = svc.anomalies(Duration::days(1)).await.expect("anomalies");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, "multi_ip_sessions");
        assert_eq!(out[0].count, 1, "one distinct user");
        assert_eq!(out[0].details["user_ids"].as_array().expect("ids").len(), 1);

        // Expire one row while still active → the expired_but_active row
        // leads the report (go order), then the multi-ip one.
        let expired_id = {
            let mut s = f.lock();
            let id = *s.sessions.keys().next().expect("row");
            s.sessions.get_mut(&id).expect("row").expires_at = Utc::now() - Duration::hours(1);
            id
        };
        let out = svc.anomalies(Duration::days(1)).await.expect("anomalies");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, "expired_but_active");
        assert_eq!(out[0].count, 1);
        assert_eq!(out[1].kind, "multi_ip_sessions");
        let _ = expired_id;
        let _ = u;
    }
}
