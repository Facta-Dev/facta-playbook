//! Postgres adapter for `OutboxRepository` (Go `platform/email` queue SQL
//! semantics 1:1: atomic claim via `FOR UPDATE SKIP LOCKED` with a 5-minute
//! `sending` lease, 2^n-minute jittered backoff, dead-letter at `max_retries`).
use crate::domain::OutboxRepository;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use <app>_core::{EmailJob, EmailMessage, Error, Result};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgOutboxRepository {
    pool: PgPool,
}

impl PgOutboxRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Map the foreign error into the domain error at the boundary.
fn map(e: sqlx::Error) -> Error {
    match e {
        sqlx::Error::RowNotFound => Error::NotFound,
        other => Error::Internal(other.to_string()),
    }
}

#[derive(Debug, sqlx::FromRow)]
struct JobRow {
    id: uuid::Uuid,
    to_email: String,
    template_key: String,
    payload: serde_json::Value,
    status: String,
    retry_count: i32,
    max_retries: i32,
    scheduled_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

impl From<JobRow> for EmailJob {
    fn from(r: JobRow) -> Self {
        Self {
            id: r.id,
            to: r.to_email,
            template_key: r.template_key,
            vars: r.payload,
            status: r.status,
            retry_count: r.retry_count,
            max_retries: r.max_retries,
            scheduled_at: r.scheduled_at,
            sent_at: r.sent_at,
            last_error: r.last_error,
        }
    }
}
#[async_trait]
impl OutboxRepository for PgOutboxRepository {
    async fn enqueue(&self, message: &EmailMessage) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_jobs (id, to_email, template_key, payload) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(uuid::Uuid::now_v7())
        .bind(&message.to)
        .bind(&message.template_key)
        .bind(&message.vars)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn claim(&self, now: DateTime<Utc>, limit: i64) -> Result<Vec<EmailJob>> {
        let rows = sqlx::query_as::<_, JobRow>(
            "WITH picked AS ( \
                 SELECT id FROM email_jobs \
                 WHERE (status = 'queued' AND scheduled_at <= $1) \
                    OR (status = 'sending' AND scheduled_at <= $1 - interval '5 minutes') \
                 ORDER BY scheduled_at LIMIT $2 FOR UPDATE SKIP LOCKED \
             ) \
             UPDATE email_jobs j SET status = 'sending' \
             WHERE j.id IN (SELECT id FROM picked) \
             RETURNING j.id, j.to_email, j.template_key, j.payload, j.status, \
                       j.retry_count, j.max_retries, j.scheduled_at, \
                       j.sent_at, j.last_error",
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn complete(&self, job_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE email_jobs SET status = 'sent', sent_at = now(), \
             last_error = NULL WHERE id = $1",
        )
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }

    async fn fail(&self, job: &EmailJob, cause: &str) -> Result<()> {
        let n = i64::from(job.retry_count) + 1;
        let backoff_secs = backoff_secs_of(n);
        // Jitter +/-10% OF THE BACKOFF (never of the wall clock) so
        // simultaneous failures do not re-synchronize polling. ponytail: a
        // deterministic hash stands in for go's math/rand — no rand dep.
        let jitter_secs = jitter_secs(job.id, n);
        let scheduled = Utc::now() + chrono::Duration::seconds(backoff_secs + jitter_secs);
        let status = if n >= i64::from(job.max_retries) {
            "failed"
        } else {
            "queued"
        };
        sqlx::query(
            "UPDATE email_jobs SET status = $2, retry_count = $3, \
             last_error = $4, scheduled_at = $5 WHERE id = $1",
        )
        .bind(job.id)
        .bind(status)
        .bind(i32::try_from(n).unwrap_or(i32::MAX))
        .bind(cause)
        .bind(scheduled)
        .execute(&self.pool)
        .await
        .map_err(map)?;
        Ok(())
    }
}

/// Jitter in [-backoff/10, +backoff/10): a stable hash of (job id, attempt)
/// — spreads simultaneous failures without a rand dependency.
fn jitter_secs(job_id: uuid::Uuid, attempt: i64) -> i64 {
    // ±10% OF THE BACKOFF, always nonnegative modulo against the u64 hash.
    let span = (backoff_secs_of(attempt) / 5).max(1);
    let mut h = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&job_id, &mut h);
    std::hash::Hash::hash(&attempt, &mut h);
    let raw = (std::hash::Hasher::finish(&h) % span.cast_unsigned()).cast_signed();
    raw - span / 2
}

fn backoff_secs_of(attempt: i64) -> i64 {
    (1i64 << attempt.min(30)).saturating_mul(60)
}
