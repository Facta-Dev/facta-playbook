//! Ports + adapters. Traits in `domain`, Postgres impls in `adapters`,
//! test doubles in `mock`.
pub mod adapters;
pub mod domain;
#[cfg(any(test, feature = "mock"))]
pub mod mock;

pub use adapters::{
    PgAuditLogRepository, PgEmailVerificationRepository, PgOutboxRepository,
    PgPasswordResetRepository, PgSessionRepository, PgUserRepository, PgWidgetRepository,
};
pub use domain::{
    AuditLog, AuditLogRepository, EmailVerificationRepository, OutboxRepository,
    PasswordResetRepository, SessionRepository, SessionUser, UpdateMeInput, UserPatch,
    UserRepository, WidgetRepository,
};
