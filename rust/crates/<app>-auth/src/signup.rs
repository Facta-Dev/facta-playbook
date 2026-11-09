//! Signup / verify / resend — go `authsignup` usecase.go parity, adapted to
//! the landed `core`/`ports`/`warden` APIs (422s read as
//! [`Error::Invalid`], 400).
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;

use <app>_core::{EmailMessage, Error, Result, Role, User, UserId, UserType, Verification};
use <app>_ports::{EmailVerificationRepository, OutboxRepository, UserRepository};
use <app>_warden::{constant_time_eq, hash_password, new_code};

/// go `maxCodeAttempts` (authsignup domain.go): past five wrong-code guesses
/// the code reads as invalid (24h of guessing cannot exhaust a 6-digit space).
const MAX_CODE_ATTEMPTS: i32 = 5;

/// Signup request (go `SignupInput`): `user_type` is optional — `""` resolves
/// to [`UserType::Company`], else one of `ADMIN|COMPANY|USER`
/// case-insensitive; anything else is invalid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignupInput {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
    pub language: String,
    pub user_type: String,
}

/// Signup use-case: unverified accounts + 6-digit codes (go parity,
/// idempotent re-signup re-issues the code).
pub struct SignupService {
    users: Arc<dyn UserRepository>,
    verifications: Arc<dyn EmailVerificationRepository>,
    outbox: Arc<dyn OutboxRepository>,
    pepper: String,
    verify_ttl: Duration,
}

impl std::fmt::Debug for SignupService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignupService").finish_non_exhaustive()
    }
}

impl SignupService {
    /// Wires the services; constructor signatures are pinned by the api
    /// crate's `state.rs` compose/`with_repos`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        users: Arc<dyn UserRepository>,
        verifications: Arc<dyn EmailVerificationRepository>,
        outbox: Arc<dyn OutboxRepository>,
        pepper: &str,
        verify_ttl: Duration,
    ) -> Self {
        Self {
            users,
            verifications,
            outbox,
            pepper: pepper.to_owned(),
            verify_ttl,
        }
    }

    /// Signup: normalize, validate, create-or-reuse, re-issue the code (go
    /// parity: an unverified re-signup is idempotent and re-sends a fresh
    /// code; a verified address conflicts).
    ///
    /// # Errors
    /// [`Error::Invalid`] ("invalid request") when the email is empty, the
    /// password is under 8 chars or the user type does not normalize;
    /// [`Error::Conflict`] ("email already registered") when the address is
    /// verified; port/store errors as raised.
    pub async fn signup(&self, input: SignupInput) -> Result<()> {
        let email = input.email.trim().to_lowercase();
        if email.is_empty() || input.password.len() < 8 {
            return Err(Error::Invalid("invalid request".into()));
        }
        let Some(user_type) = normalize_user_type(&input.user_type) else {
            return Err(Error::Invalid("invalid request".into()));
        };
        let now = Utc::now();
        let existing = self.users.get_user_by_email(&email).await?;
        if let Some(existing) = &existing
            && existing.is_verified
        {
            return Err(Error::Conflict("email already registered".into()));
        }
        // Re-issue regardless of path: one pending code, always freshly sent.
        let (user_id, lang) = if let Some(existing) = existing {
            (existing.id, default_lang(&existing.language))
        } else {
            let hash = hash_password(&input.password, &self.pepper)?;
            let row = User {
                id: UserId::new(),
                email: email.clone(),
                first_name: input.first_name.clone(),
                last_name: input.last_name.clone(),
                role: Role::Viewer,
                user_type,
                is_active: true,
                is_blocked: false,
                is_verified: false,
                retry_count: 0,
                password: hash,
                language: default_lang(&input.language),
                profile_picture: None,
                google_id: None,
                github_id: None,
                last_login: None,
                created_at: now,
                updated_at: now,
            };
            self.users.create_user(&row).await?;
            self.create_profile(row.id, user_type, &input.first_name, &input.last_name)
                .await?;
            (row.id, row.language.clone())
        };
        self.issue_code(user_id, &email, &input.first_name, &lang)
            .await
    }

    /// Consumes the 6-digit code and flips `is_verified`. Wrong codes bump the
    /// attempt counter; past [`MAX_CODE_ATTEMPTS`] the code reads as invalid.
    ///
    /// # Errors
    /// [`Error::Invalid`] with the go message: "verification code
    /// invalid"/"already used"/"expired".
    pub async fn verify_email(&self, email: &str, code: &str) -> Result<()> {
        let email = email.trim().to_lowercase();
        if code.is_empty() {
            return Err(Error::Invalid("verification code invalid".into()));
        }
        let Some(user) = self.users.get_user_by_email(&email).await? else {
            return Err(Error::Invalid("verification code invalid".into()));
        };
        let Some(v) = self.verifications.latest_verification(user.id).await? else {
            return Err(Error::Invalid("verification code invalid".into()));
        };
        if v.used {
            return Err(Error::Invalid("verification code already used".into()));
        }
        if Utc::now() > v.expires_at {
            return Err(Error::Invalid("verification code expired".into()));
        }
        if v.attempts >= MAX_CODE_ATTEMPTS {
            return Err(Error::Invalid("verification code invalid".into()));
        }
        if !constant_time_eq(&v.token, code) {
            self.verifications.bump_verification(v.id).await?;
            return Err(Error::Invalid("verification code invalid".into()));
        }
        if !self.verifications.use_verification(v.id, &v.token).await? {
            return Err(Error::Invalid("verification code already used".into()));
        }
        self.verifications.set_email_verified(user.id).await
    }

    /// Fresh code for an unverified user; unknown and already-verified
    /// emails answer the same (no existence leak).
    ///
    /// # Errors
    /// Port/store errors as raised.
    pub async fn resend_verification(&self, email: &str) -> Result<()> {
        let email = email.trim().to_lowercase();
        let Some(user) = self.users.get_user_by_email(&email).await? else {
            return Ok(());
        };
        if user.is_verified {
            return Ok(());
        }
        self.issue_code(user.id, &user.email, &user.first_name, &user.language)
            .await
    }

    /// One pending code + the `verify_email` email (go enqueueVerification:
    /// `first_name`/`code`/`locale` vars; the signup path passes the REQUEST's
    /// first name, the resend path the stored one).
    async fn issue_code(
        &self,
        user_id: UserId,
        email: &str,
        first_name: &str,
        lang: &str,
    ) -> Result<()> {
        let code = new_code(6)?;
        let verification = Verification {
            id: uuid::Uuid::now_v7(),
            user_id,
            token: code.clone(),
            expires_at: Utc::now() + self.verify_ttl,
            used: false,
            attempts: 0,
        };
        self.verifications
            .create_verification(&verification)
            .await?;
        self.enqueue_verification(email, first_name, lang, &code)
            .await
    }

    /// go `enqueueVerification` parity (template `verify_email`).
    async fn enqueue_verification(
        &self,
        email: &str,
        first_name: &str,
        lang: &str,
        code: &str,
    ) -> Result<()> {
        self.outbox.enqueue(&EmailMessage {
            to: email.to_owned(),
            template_key: "verify_email".to_owned(),
            vars: json!({ "first_name": first_name, "code": code, "locale": default_lang(lang) }),
        })
        .await
    }

    /// Seeds the 1:1 profile row for the user's type; ADMIN needs none.
    async fn create_profile(
        &self,
        user_id: UserId,
        user_type: UserType,
        first_name: &str,
        last_name: &str,
    ) -> Result<()> {
        let name = format!("{first_name} {last_name}").trim().to_owned();
        match user_type {
            UserType::User => self.verifications.create_user_profile(user_id, &name).await,
            UserType::Company => {
                self.verifications
                    .create_company_profile(user_id, &name)
                    .await
            }
            UserType::Admin => Ok(()), // no profile table by design
        }
    }
}

/// go `NormalizeUserType`: `""` → COMPANY; else ADMIN|COMPANY|USER,
/// case-insensitive; anything else is `None`.
#[must_use]
fn normalize_user_type(t: &str) -> Option<UserType> {
    match t.trim().to_uppercase().as_str() {
        "" | "COMPANY" => Some(UserType::Company),
        "ADMIN" => Some(UserType::Admin),
        "USER" => Some(UserType::User),
        _ => None,
    }
}

/// go `defaultLang`: empty → "en".
#[must_use]
fn default_lang(lang: &str) -> String {
    if lang.is_empty() {
        "en".to_owned()
    } else {
        lang.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::Fakes;
    use <app>_warden::verify_password;

    const PEPPER: &str = "test-pepper";

    fn service(f: &Fakes) -> SignupService {
        SignupService::new(
            f.users.clone(),
            f.verifications.clone(),
            f.outbox.clone(),
            PEPPER,
            Duration::hours(24),
        )
    }

    fn input(email: &str) -> SignupInput {
        SignupInput {
            email: email.to_owned(),
            first_name: "Ada".to_owned(),
            last_name: "Lovelace".to_owned(),
            password: "password123".to_owned(),
            language: String::new(),
            user_type: String::new(),
        }
    }

    /// The 6-digit code of the one enqueued `verify_email` job.
    #[track_caller]
    fn code_of(f: &Fakes) -> String {
        let vars = f.lock().emails.first().expect("one email").vars.clone();
        vars["code"].as_str().expect("code var").to_owned()
    }

    #[tokio::test]
    async fn signup_creates_unverified_user_profile_and_code() {
        let f = Fakes::new();
        service(&f)
            .signup(input("ada@t.dev"))
            .await
            .expect("signup ok");
        let s = f.lock();
        assert_eq!(s.users.len(), 1);
        let user = s.users.values().next().expect("created").clone();
        assert!(user.is_active && !user.is_verified && !user.is_blocked);
        assert_eq!(user.user_type, UserType::Company); // blank resolves to COMPANY
        assert_eq!(user.role, Role::Viewer);
        assert!(verify_password(&user.password, "password123", PEPPER));
        assert_eq!(s.profiles[0], ("companies", user.id));
        assert_eq!(s.emails.len(), 1);
        let email = &s.emails[0];
        assert_eq!(email.to, "ada@t.dev");
        assert_eq!(email.template_key, "verify_email");
        assert_eq!(email.vars["first_name"], "Ada");
        assert_eq!(email.vars["locale"], "en");
        let (token, attempts, is_used) = {
            let v = s.verifications[0].clone();
            (v.token, v.attempts, v.used)
        };
        drop(s);
        assert_eq!(code_of(&f), token);
        assert_eq!(token.len(), 6);
        assert!(!is_used && attempts == 0);
    }

    #[tokio::test]
    async fn signup_validates_email_password_and_user_type() {
        let f = Fakes::new();
        let svc = service(&f);
        let mut i = input("ada@t.dev");
        i.email = "   ".to_owned();
        let err = svc.signup(i).await.unwrap_err();
        assert_eq!(err.status_code(), 400);
        assert!(matches!(err, Error::Invalid(m) if m == "invalid request"));

        let mut i = input("ada@t.dev");
        i.password = "short".to_owned();
        assert!(svc.signup(i).await.is_err());

        let mut i = input("ada@t.dev");
        i.user_type = "nope".to_owned();
        let err = svc.signup(i).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
        // Nothing was written on any invalid path.
        assert!(f.lock().users.is_empty());
        assert_eq!(format!("{err:?}"), "Invalid(\"invalid request\")");
    }

    #[tokio::test]
    async fn signup_normalizes_user_type_and_email() {
        let f = Fakes::new();
        let svc = service(&f);
        let mut i = input("  ADA@T.dev ");
        i.user_type = "user".to_owned();
        svc.signup(i).await.expect("ok");
        {
            let s = f.lock();
            let user = s.users.values().next().expect("created").clone();
            assert_eq!(user.email, "ada@t.dev"); // lowercased + trimmed
            assert_eq!(user.user_type, UserType::User);
            assert_eq!(s.profiles[0], ("users_profile", user.id));
            drop(s);
        }
        // ADMIN: no profile row at all.
        let f = Fakes::new();
        let mut i = input("root@t.dev");
        i.user_type = "ADMIN".to_owned();
        service(&f).signup(i).await.expect("ok");
        assert!(f.lock().profiles.is_empty());
        assert_eq!(
            f.lock().users.values().next().expect("u").user_type,
            UserType::Admin
        );
    }

    #[tokio::test]
    async fn re_signup_reissues_the_code_without_reseeding_profiles() {
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("first ok");
        let id_before = f.lock().users.values().next().expect("u").id;
        svc.signup(input("ada@t.dev")).await.expect("second ok");
        let s = f.lock();
        assert_eq!(s.users.len(), 1, "id + language reused");
        assert_eq!(s.users.values().next().expect("u").id, id_before);
        assert_eq!(s.verifications.len(), 2, "fresh code re-issued");
        assert_eq!(s.emails.len(), 2);
        assert_eq!(s.profiles.len(), 1, "no profile double-seed");
        // The re-issue carries the REQUEST's first name (go parity).
        assert_eq!(s.emails[1].vars["first_name"], "Ada");
        drop(s);
    }

    #[tokio::test]
    async fn verified_address_is_a_conflict() {
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("ok");
        let code = code_of(&f);
        svc.verify_email("ada@t.dev", &code)
            .await
            .expect("verified");
        let err = svc.signup(input("ada@t.dev")).await.unwrap_err();
        assert_eq!(err.status_code(), 409);
        assert!(matches!(err, Error::Conflict(m) if m == "email already registered"));
        // And a verified re-signup enqueued nothing new.
        assert_eq!(f.lock().emails.len(), 1);
    }

    #[tokio::test]
    async fn verify_email_checks_state_then_bumps_wrong_guesses() {
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("ok");
        let code = code_of(&f);

        // Empty code / unknown user read as invalid.
        let err = svc.verify_email("ada@t.dev", "").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code invalid"));
        let err = svc.verify_email("ghost@t.dev", "000000").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code invalid"));

        // Wrong code: attempts bump, still unverified, still usable.
        let err = svc.verify_email("ada@t.dev", "999999").await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code invalid"));
        {
            let s = f.lock();
            let v = s.verifications[0].clone();
            drop(s);
            assert_eq!(v.attempts, 1);
            assert!(!v.used);
        }

        // A correct code still verifies at attempts < 5 (go check order).
        svc.verify_email("ada@t.dev", &code)
            .await
            .expect("verified");
        {
            let s = f.lock();
            assert!(s.users.values().next().expect("u").is_verified);
            assert!(s.verifications[0].used);
            drop(s);
        }

        // Replay reads as used.
        let err = svc.verify_email("ada@t.dev", &code).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code already used"));
    }

    #[tokio::test]
    async fn verify_email_rejects_expired_and_exhausted_codes() {
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("ok");
        let code = code_of(&f);
        // Expired: shift the newest verification into the past.
        f.lock().verifications.last_mut().expect("v").expires_at =
            Utc::now() - Duration::seconds(1);
        let err = svc.verify_email("ada@t.dev", &code).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code expired"));

        // Exhausted: five wrong guesses, then even the CORRECT code is dead
        // (go: attempts >= maxCodeAttempts precedes the token compare).
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("ok");
        let code = code_of(&f);
        for _ in 0..=MAX_CODE_ATTEMPTS {
            let _ = svc.verify_email("ada@t.dev", "111111").await;
        }
        let attempts = f.lock().verifications[0].attempts;
        assert_eq!(attempts, 5, "attempt 5+ stops bumping (state read first)");
        let err = svc.verify_email("ada@t.dev", &code).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m == "verification code invalid"));
    }

    #[tokio::test]
    async fn resend_answers_the_same_for_unknown_and_verified() {
        let f = Fakes::new();
        let svc = service(&f);
        svc.resend_verification("ghost@t.dev")
            .await
            .expect("ok noop");
        assert!(f.lock().emails.is_empty(), "no existence leak");
        // Verified user: noop too.
        svc.signup(input("ada@t.dev")).await.expect("ok");
        let code = code_of(&f);
        svc.verify_email("ada@t.dev", &code)
            .await
            .expect("verified");
        svc.resend_verification("ada@t.dev").await.expect("ok noop");
        assert_eq!(f.lock().emails.len(), 1, "no re-send after verified");
        // Unverified: a fresh code + email.
        let f = Fakes::new();
        let svc = service(&f);
        svc.signup(input("ada@t.dev")).await.expect("ok");
        svc.resend_verification("ADA@T.dev ").await.expect("ok");
        {
            let s = f.lock();
            assert_eq!(s.verifications.len(), 2);
            assert_eq!(s.emails.len(), 2);
            drop(s);
        }
    }
}
