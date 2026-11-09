//! Durable outbox contract (Go `email.Queue`): slices enqueue
//! [`<app>_core::EmailMessage`]s into `email_jobs`; the loom worker claims
//! rows (`FOR UPDATE SKIP LOCKED`), renders and delivers.
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{EmailJob, EmailMessage, Result};

#[async_trait]
pub trait OutboxRepository: Send + Sync + 'static {
    /// Insert one queued job (Go `Enqueue`); `id/scheduled_at/max_retries` are
    /// column defaults, the payload stores `vars` as JSONB.
    async fn enqueue(&self, message: &EmailMessage) -> Result<()>;
    /// Lock up to `limit` due rows and flip them to `sending` (Go `Claim`).
    /// Rows already `sending` become claimable again after the 5-minute
    /// lease (crashed-worker recovery, at-least-once).
    async fn claim(&self, now: DateTime<Utc>, limit: i64) -> Result<Vec<EmailJob>>;
    /// Mark the job delivered (Go `Complete`).
    async fn complete(&self, job_id: uuid::Uuid) -> Result<()>;
    /// Record the failure (Go `Fail`): the adapter bumps `retry_count` with
    /// 2^n-minute backoff and dead-letters (status `failed`) once
    /// `retry_count >= max_retries`.
    async fn fail(&self, job: &EmailJob, cause: &str) -> Result<()>;
}
