//! Postgres adapters — the ONLY place SQL lives.
pub mod audit_log_pg;
pub mod email_verification_pg;
pub mod outbox_pg;
pub mod password_reset_pg;
pub mod session_pg;
pub mod user_pg;
pub mod widget_pg;

pub use audit_log_pg::PgAuditLogRepository;
pub use email_verification_pg::PgEmailVerificationRepository;
pub use outbox_pg::PgOutboxRepository;
pub use password_reset_pg::PgPasswordResetRepository;
pub use session_pg::PgSessionRepository;
pub use user_pg::PgUserRepository;
pub use widget_pg::PgWidgetRepository;
