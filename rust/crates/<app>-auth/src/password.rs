//! Password flows over email codes — go `authpassword` usecase.go parity,
//! adapted to the landed `core`/`ports`/`warden` APIs (go 422s read as
//! [`Error::Invalid`], 400; the go 429 reads as
//! [`Error::RateLimited`]).
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;

use <app>_core::{EmailMessage, Error, PasswordReset, Result, SessionId, User, UserId};
use <app>_ports::{OutboxRepository, PasswordResetRepository, UserRepository};
use <app>_warden::{constant_time_eq, hash_password, new_code, verify_password};

/// go `maxCodeAttempts` (authpassword domain.go) — with a 6-digit space and a
/// 30m window, validate/reset must not be an unthrottled oracle.
const MAX_CODE_ATTEMPTS: i32 = 5;

/// Password use-case: forgot (rate-limited, no existence leak), validate,
/// reset (auto-verify + optional revoke-all), change (current password
/// re-verified; other sessions revoked, caller's session survives via
/// `jti == sessions.id`).
pub struct PasswordService {
    users: Arc<dyn UserRepository>,
    resets: Arc<dyn PasswordResetRepository>,
    outbox: Arc<dyn OutboxRepository>,
    pepper: String,
    reset_ttl: Duration,
    rate_window: Duration,
    rate_limit: i32,
    log_out_all_on_reset: bool,
}

impl std::fmt::Debug for PasswordService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordService").finish_non_exhaustive()
    }
}

impl PasswordService {
    /// Wires the service (api `state.rs` contract).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepository>,
        resets: Arc<dyn PasswordResetRepository>,
        outbox: Arc<dyn OutboxRepository>,
        pepper: &str,
        reset_ttl: Duration,
        rate_window: Duration,
        rate_limit: i32,
        log_out_all_on_reset: bool,
    ) -> Self {
        Self {
            users,
            resets,
            outbox,
            pepper: pepper.to_owned(),
            reset_ttl,
            rate_window,
            rate_limit,
            log_out_all_on_reset,
        }
    }

    /// Request a reset code: rate-limited per email, unknown addresses read
    /// as success (no existence leak).
    ///
    /// # Errors
    /// [`Error::RateLimited`] when [`Self::rate_limit`] codes were already
    /// created inside the window; port errors as raised.
    pub async fn forgot(&self, email: &str) -> Result<()> {
        let email = email.trim().to_lowercase();
        let now = Utc::now();
        let n = self
            .resets
            .count_resets_since(&email, now - self.rate_window)
            .await?;
        if n >= i64::from(self.rate_limit) {
            return Err(Error::RateLimited {
                retry_after_secs: u32::try_from(self.rate_window.num_seconds().max(1))
                    .unwrap_or(u32::MAX),
            });
        }
        let Some(user) = self.users.get_user_by_email(&email).await? else {
            return Ok(());
        };
        let code = new_code(6)?;
        let reset = PasswordReset {
            id: uuid::Uuid::now_v7(),
            user_id: user.id,
            email: email.clone(),
            token: code.clone(),
            expires_at: now + self.reset_ttl,
            used: false,
            attempts: 0,
        };
        self.resets.create_reset(&reset).await?;
        self.enqueue(&user, "reset_password", &code).await
    }

    /// Checks a code without consuming it (go `Validate`).
    ///
    /// # Errors
    /// [`Error::Invalid`] with the go messages: "reset code
    /// invalid"/"already used"/"expired"; wrong guesses bump the attempt
    /// counter.
    pub async fn validate(&self, email: &str, code: &str) -> Result<()> {
        let email = email.trim().to_lowercase();
        self.check(&email, code).await.map(|_| ())
    }

    /// Consumes the code atomically, sets the new password, auto-verifies
    /// the account and (optionally) revokes every active session (go
    /// `logOutAllOnReset`).
    ///
    /// # Errors
    /// [`Error::Invalid`] ("invalid request" under 8 chars; the go code
    /// errors otherwise); port errors as raised.
    pub async fn reset(&self, email: &str, code: &str, new_password: &str) -> Result<()> {
        let email = email.trim().to_lowercase();
        if new_password.len() < 8 {
            return Err(Error::Invalid("invalid request".into()));
        }
        let r = self.check(&email, code).await?;
        let Some(user) = self.users.get_user_by_email(&email).await? else {
            return Err(Error::Invalid("reset code invalid".into()));
        };
        let hash = hash_password(new_password, &self.pepper)?;
        if !self.resets.mark_reset_used(r.id, &r.token).await? {
            return Err(Error::Invalid("reset code already used".into()));
        }
        self.resets.set_password_and_verify(user.id, &hash).await?;
        if self.log_out_all_on_reset {
            self.resets.revoke_all_sessions(user.id).await?;
        }
        self.enqueue(&user, "password_changed", "").await
    }

    /// Verifies the current password, then rotates it and revokes every
    /// OTHER active session — the caller's own session survives via
    /// `jti == sessions.id` (go `Change`).
    ///
    /// # Errors
    /// [`Error::Invalid`] ("invalid request" under 8 chars);
    /// [`Error::Unauthenticated`] ("current password is wrong") on an
    /// unknown user or a wrong current password; port errors as raised.
    pub async fn change(
        &self,
        user_id: UserId,
        session_id: SessionId,
        current: &str,
        new_password: &str,
    ) -> Result<()> {
        if new_password.len() < 8 {
            return Err(Error::Invalid("invalid request".into()));
        }
        let Some(user) = self.users.get_user_by_id(user_id).await? else {
            return Err(Error::Unauthenticated("current password is wrong".into()));
        };
        if !verify_password(&user.password, current, &self.pepper) {
            return Err(Error::Unauthenticated("current password is wrong".into()));
        }
        let hash = hash_password(new_password, &self.pepper)?;
        self.resets.set_password(user.id, &hash).await?;
        self.resets
            .revoke_other_sessions(user.id, session_id)
            .await?;
        self.enqueue(&user, "password_changed", "").await
    }

    /// Loads the latest reset row for the email, validates state and (on a
    /// wrong code) bumps the attempt counter; constant-time token compare.
    async fn check(&self, email: &str, code: &str) -> Result<PasswordReset> {
        let Some(r) = self.resets.latest_reset(email).await? else {
            return Err(Error::Invalid("reset code invalid".into()));
        };
        if r.used {
            return Err(Error::Invalid("reset code already used".into()));
        }
        if Utc::now() > r.expires_at {
            return Err(Error::Invalid("reset code expired".into()));
        }
        if r.attempts >= MAX_CODE_ATTEMPTS {
            return Err(Error::Invalid("reset code invalid".into()));
        }
        if !constant_time_eq(&r.token, code) {
            self.resets.bump_reset(r.id).await?;
            return Err(Error::Invalid("reset code invalid".into()));
        }
        Ok(r)
    }

    /// Renders (per user language) and queues one email job (go `enqueue`).
    async fn enqueue(&self, user: &User, template: &str, code: &str) -> Result<()> {
        let locale = if user.language.is_empty() {
            "en".to_owned()
        } else {
            user.language.clone()
        };
        self.outbox
            .enqueue(&EmailMessage {
                to: user.email.clone(),
                template_key: template.to_owned(),
                vars: json!({
                    "first_name": user.first_name,
                    "code": code,
                    "locale": locale,
                }),
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{Fakes, fixture};

    const PEPPER: &str = "test-pepper";

    fn service(f: &Fakes) -> PasswordService {
        PasswordService::new(
            f.users.clone(),
            f.resets.clone(),
            f.outbox.clone(),
            PEPPER,
            Duration::minutes(30),
            Duration::minutes(5),
            2,
            true,
        )
    }

    /// Seed a verified user with a real argon2 password.
    fn seed(f: &Fakes, email: &str) -> User {
        let mut u = fixture::user(1, email);
        u.is_verified = true;
        u.password = hash_password("old-password", PEPPER).expect("hash");
        let id = u.id;
        f.lock().users.insert(id, u.clone());
        u
    }

    /// The 6-digit code of the one enqueued `reset_password` job.
    #[track_caller]
    fn code_of(f: &Fakes) -> String {
        let vars = f.lock().emails.last().expect("email").vars.clone();
        vars["code"].as_str().expect("code var").to_owned()
    }

    #[tokio::test]
    async fn forgot_is_rated_no_leak_and_posts_the_code() {
        let f = Fakes::new();
        let svc = service(&f);

        // Unknown address: success, nothing stored (no existence leak).
        svc.forgot("ghost@t.dev").await.expect("noop");
        {
            let s = f.lock();
            assert!(s.resets.is_empty() && s.emails.is_empty());
            drop(s);
        }

        // Two codes in the window pass; the third is rate-limited (429).
        seed(&f, "ada@t.dev");
        svc.forgot(" ada@T.dev ").await.expect("first");
        svc.forgot("ada@t.dev").await.expect("second");
        let err = svc.forgot("ada@t.dev").await.unwrap_err();
        assert_eq!(err.status_code(), 429);
        assert!(matches!(
            err,
            Error::RateLimited {
                retry_after_secs: 300
            }
        ));
        assert_eq!(f.lock().resets.len(), 2, "refusal created nothing");

        // The email job carries the code + the stored first name.
        let code = code_of(&f);
        assert_eq!(code.len(), 6);
        let s = f.lock();
        let email = s.emails.last().expect("email").clone();
        assert_eq!(email.to, "ada@t.dev");
        assert_eq!(email.template_key, "reset_password");
        assert_eq!(email.vars["first_name"], "Ada");
        assert_eq!(email.vars["locale"], "en");
        drop(s);
        let row = f.lock().resets.last().expect("row").0.clone();
        assert_eq!(row.token, code);
        assert!(Utc::now() < row.expires_at, "30m ahead of now");
    }

    #[tokio::test]
    async fn validate_bumps_wrong_guesses_without_consuming() {
        let f = Fakes::new();
        let svc = service(&f);
        seed(&f, "ada@t.dev");
        svc.forgot("ada@t.dev").await.expect("forgot");
        let code = code_of(&f);

        // Unknown reset target: invalid.
        let err = svc.validate("ghost@t.dev", "000000").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code invalid"));
        // Wrong code: attempt bump, no consumption.
        let err = svc.validate("ada@t.dev", "111111").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code invalid"));
        assert_eq!(f.lock().resets[0].0.attempts, 1);

        // The correct code passes but does NOT consume (go validate).
        svc.validate("ada@t.dev", &code).await.expect("valid");
        assert!(!f.lock().resets[0].0.used);

        // Past 5 wrong guesses even the correct code reads as invalid.
        for _ in 0..5 {
            let _ = svc.validate("ada@t.dev", "222222").await;
        }
        let attempts = f.lock().resets[0].0.attempts;
        assert_eq!(attempts, 5, "bumping stops at the cap");
        let err = svc.validate("ada@t.dev", &code).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code invalid"));
    }

    #[tokio::test]
    async fn reset_consumes_verifies_revokes_and_stamps() {
        let f = Fakes::new();
        let svc = service(&f);
        let u = seed(&f, "ada@t.dev");
        // An active session to prove the log-out-all-on-reset sweep.
        let row = crate::fakes::fixture::session(1, 1, "refresh-a");
        f.lock().sessions.insert(row.id, row);

        // Short passwords are refused before the code is touched.
        let err = svc.reset("ada@t.dev", "000000", "short").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "invalid request"));

        svc.forgot("ada@t.dev").await.expect("forgot");
        let code = code_of(&f);

        // The unknown-user path reads as invalid (defensive).
        let err = svc
            .reset("ghost@t.dev", code_of(&f).as_str(), "new-password-1")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code invalid"));
        let _ = &u;

        // A wrong code bumps attempts and refuses.
        let err = svc
            .reset("ada@t.dev", "333333", "new-password-1")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code invalid"));

        // Expired code reads as expired.
        {
            let mut s = f.lock();
            let mut expired = s.resets.last().expect("row").0.clone();
            expired.expires_at = Utc::now() - Duration::seconds(1);
            s.resets.pop();
            s.resets.push((expired, Utc::now()));
        }
        let err = svc
            .reset("ada@t.dev", &code, "new-password-1")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code expired"));

        // Fresh code after the expired one (still under the rate limit),
        // then the happy path.
        svc.forgot("ada@t.dev").await.expect("second forgot");
        let code = code_of(&f);
        svc.reset("ada@t.dev", &code, "new-password-1")
            .await
            .expect("reset");

        let user = {
            let s = f.lock();
            let user = s.users.get(&u.id).expect("u").clone();
            drop(s);
            assert!(user.is_verified, "reset auto-verifies");
            user
        };
        assert!(verify_password(&user.password, "new-password-1", PEPPER));
        // The open session was swept (log_out_all_on_reset = true).
        assert!(!f.lock().sessions.values().next().expect("row").is_active);
        // The reset row reads as used; a replay refuses as already used.
        assert!(f.lock().resets.last().expect("row").0.used);
        let err = svc
            .reset("ada@t.dev", &code, "new-password-1")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "reset code already used"));
        // The password_changed notice went out with an empty code.
        let email = f.lock().emails.last().expect("email").clone();
        assert_eq!(email.template_key, "password_changed");
        assert_eq!(email.vars["code"], "");
    }

    #[tokio::test]
    async fn change_keeps_the_callers_session_alive() {
        let f = Fakes::new();
        let svc = service(&f);
        let u = seed(&f, "ada@t.dev");
        let keep = crate::fakes::fixture::session(1, 1, "keep-me");
        let other = crate::fakes::fixture::session(2, 1, "kill-me");
        let keep_id = keep.id;
        let other_id = other.id;
        f.lock().sessions.insert(keep.id, keep);
        f.lock().sessions.insert(other.id, other);

        // Unknown user and wrong current password both read 401.
        let err = svc
            .change(
                crate::fakes::uid(9),
                crate::fakes::sid(1),
                "old-password",
                "new-password-1",
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "current password is wrong"
        ));
        let err = svc
            .change(
                u.id,
                crate::fakes::sid(1),
                "wrong-current",
                "new-password-1",
            )
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            Error::Unauthenticated(m) if m == "current password is wrong"
        ));

        // Short password: invalid request, nothing changed.
        let err = svc
            .change(u.id, keep_id, "old-password", "short")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "invalid request"));

        // Happy path: new hash stored, OTHER session deactivated, the
        // caller's own row survives (jti == sessions.id).
        svc.change(u.id, keep_id, "old-password", "new-password-1")
            .await
            .expect("change");
        let s = f.lock();
        let user = s.users.get(&u.id).expect("u").clone();
        assert!(verify_password(&user.password, "new-password-1", PEPPER));
        assert!(s.sessions.get(&keep_id).expect("keep").is_active);
        assert!(!s.sessions.get(&other_id).expect("other").is_active);
        drop(s);
        let email = f.lock().emails.last().expect("email").clone();
        assert_eq!(email.template_key, "password_changed");
        assert_eq!(email.vars["code"], "");
    }
}
