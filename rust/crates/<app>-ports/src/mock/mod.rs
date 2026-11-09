//! In-memory test doubles for the port traits. Compiled only under
//! `#[cfg(any(test, feature = "mock"))]` (see `lib.rs`).
pub mod audit_log_mock;
pub mod email_verification_mock;
pub mod outbox_mock;
pub mod password_reset_mock;
pub mod session_mock;
pub mod user_mock;
pub mod widget_mock;

pub use audit_log_mock::MockAuditLogRepository;
pub use email_verification_mock::MockEmailVerificationRepository;
pub use outbox_mock::MockOutboxRepository;
pub use password_reset_mock::MockPasswordResetRepository;
pub use session_mock::MockSessionRepository;
pub use user_mock::MockUserRepository;
pub use widget_mock::MockWidgetRepository;

/// Shared test fixture: a `core::User` from the pinned field set.
/// Deterministic ids (`from_u128`) keep ordering assertions stable; the fn is
/// test-only (all users live in `cfg(test)` modules).
#[cfg(test)]
pub(crate) fn test_user(id: u64, email: &str) -> <app>_core::User {
    use <app>_core::{UserId, UserType};
    let now = chrono::Utc::now();
    <app>_core::User {
        id: UserId(uuid::Uuid::from_u128(u128::from(id))),
        email: email.to_owned(),
        first_name: "First".to_owned(),
        last_name: "Last".to_owned(),
        role: <app>_core::Role::Viewer,
        user_type: UserType::User,
        is_active: true,
        is_blocked: false,
        is_verified: false,
        retry_count: 0,
        password: "hash".to_owned(),
        language: "en".to_owned(),
        profile_picture: None,
        google_id: None,
        github_id: None,
        last_login: None,
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::test_user;

    #[test]
    fn test_user_ids_are_deterministic_and_distinct() {
        let a = test_user(1, "a@t.dev");
        let b = test_user(2, "b@t.dev");
        assert_ne!(a.id, b.id);
        assert_eq!(a.id, test_user(1, "a@t.dev").id);
        assert_eq!(a.email, "a@t.dev");
    }
}
