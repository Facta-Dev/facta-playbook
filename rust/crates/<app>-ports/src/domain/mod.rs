//! Port traits — the contracts use-cases depend on. No I/O assumptions leak
//! through these signatures.
pub mod audit_log_repository;
pub mod email_verification_repository;
pub mod outbox_repository;
pub mod password_reset_repository;
pub mod session_repository;
pub mod user_repository;
pub mod widget_repository;

pub use audit_log_repository::{AuditLog, AuditLogRepository};
pub use email_verification_repository::EmailVerificationRepository;
pub use outbox_repository::OutboxRepository;
pub use password_reset_repository::PasswordResetRepository;
pub use session_repository::{SessionRepository, SessionUser};
pub use user_repository::{UpdateMeInput, UserPatch, UserRepository};
pub use widget_repository::WidgetRepository;
