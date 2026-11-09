//! In-memory `OutboxRepository` double — Go `platform/email` queue semantics
//! in memory: due-only claim with `sending` lease recovery, 2^n-minute
//! backoff, dead-letter at `max_retries`.
use std::sync::{Mutex, PoisonError};

use crate::domain::OutboxRepository;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use <app>_core::{EmailJob, EmailMessage, Result};

/// The SQL defaults (0001): status 'queued', `retry_count` 0, `max_retries` 3.
const MAX_RETRIES: i32 = 3;

/// In-memory `email_jobs` (insertion order = arrival order).
#[derive(Debug, Default)]
pub struct MockOutboxRepository {
    jobs: Mutex<Vec<EmailJob>>,
}

impl MockOutboxRepository {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-back helper for tests (insertion order).
    pub fn all_jobs(&self) -> Vec<EmailJob> {
        self.jobs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl OutboxRepository for MockOutboxRepository {
    async fn enqueue(&self, message: &EmailMessage) -> Result<()> {
        self.jobs.lock().unwrap().push(EmailJob {
            id: uuid::Uuid::now_v7(),
            to: message.to.clone(),
            template_key: message.template_key.clone(),
            vars: message.vars.clone(),
            status: "queued".to_owned(),
            retry_count: 0,
            max_retries: MAX_RETRIES,
            scheduled_at: Utc::now(),
            sent_at: None,
            last_error: None,
        });
        Ok(())
    }

    async fn claim(&self, now: DateTime<Utc>, limit: i64) -> Result<Vec<EmailJob>> {
        // Due rows (same two-arm WHERE as the SQL: queued, or a `sending`
        // row whose 5-minute lease expired). ORDER BY scheduled_at — a
        // (stamp, index) sort key keeps arrival order on ties; the leases +
        // read-back stay inside one guarded block (single UPDATE parity).
        let claimed: Vec<EmailJob> = {
            let mut jobs = self.jobs.lock().unwrap();
            let mut due: Vec<(DateTime<Utc>, usize)> = jobs
                .iter()
                .enumerate()
                .filter(|(_, j)| {
                    (j.status == "queued" && j.scheduled_at <= now)
                        || (j.status == "sending" && j.scheduled_at <= now - Duration::minutes(5))
                })
                .map(|(i, j)| (j.scheduled_at, i))
                .collect();
            due.sort();
            let claimed: Vec<EmailJob> = due
                .into_iter()
                .take(usize::try_from(limit.max(0)).unwrap_or(0))
                .map(|(_, i)| {
                    let job = &mut jobs[i];
                    "sending".clone_into(&mut job.status);
                    job.clone()
                })
                .collect();
            drop(jobs);
            claimed
        };
        Ok(claimed)
    }

    async fn complete(&self, job_id: uuid::Uuid) -> Result<()> {
        if let Some(j) = self
            .jobs
            .lock()
            .unwrap()
            .iter_mut()
            .find(|j| j.id == job_id)
        {
            "sent".clone_into(&mut j.status);
            j.sent_at = Some(Utc::now());
            j.last_error = None;
        }
        Ok(())
    }

    async fn fail(&self, job: &EmailJob, cause: &str) -> Result<()> {
        let n = i64::from(job.retry_count) + 1;
        let backoff = Duration::seconds((1i64 << n.min(30)).saturating_mul(60));
        // Jitter ±10% of the backoff (the double uses 0 — always inside the
        // window the SQL jitter spans; see the Pg adapter's hash stand-in).
        let scheduled = Utc::now() + backoff;
        let status = if n >= i64::from(job.max_retries) {
            "failed"
        } else {
            "queued"
        };
        if let Some(j) = self
            .jobs
            .lock()
            .unwrap()
            .iter_mut()
            .find(|j| j.id == job.id)
        {
            status.clone_into(&mut j.status);
            j.retry_count = i32::try_from(n).unwrap_or(i32::MAX);
            j.last_error = Some(cause.to_owned());
            j.scheduled_at = scheduled;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use <app>_core::EmailMessage;

    #[tokio::test]
    async fn claim_due_order_limit_and_lease_recovery() {
        let repo = MockOutboxRepository::new();
        let msg = EmailMessage {
            to: "a@t.dev".to_owned(),
            template_key: "welcome".to_owned(),
            vars: serde_json::json!({}),
        };
        repo.enqueue(&msg).await.unwrap();
        repo.enqueue(&msg).await.unwrap();
        repo.enqueue(&msg).await.unwrap();
        assert_eq!(repo.all_jobs().len(), 3);
        // ORDER BY scheduled_at: stagger the three jobs.
        let now = Utc::now();
        {
            let mut jobs = repo.jobs.lock().unwrap();
            jobs[0].scheduled_at = now + Duration::hours(2);
            jobs[1].scheduled_at = now;
            jobs[2].scheduled_at = now + Duration::hours(1);
        }
        // Only queued rows scheduled <= now are due: exactly one.
        let claimed = repo.claim(now + Duration::minutes(1), 10).await.unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, repo.all_jobs()[1].id);
        assert_eq!(claimed[0].status, "sending");
        // LIMIT caps the batch: the lease-expired `sending` row re-claims
        // first by ORDER, so jobs[2] waits another tick.
        let again = repo.claim(now + Duration::hours(3), 1).await.unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].id, claimed[0].id);
        // LIMIT 1 defers jobs[2]: it stays queued for the next tick.
        assert_eq!(repo.all_jobs()[2].status, "queued");
    }

    #[tokio::test]
    async fn stale_sending_rows_are_recovered_after_the_lease() {
        let repo = MockOutboxRepository::new();
        let msg = EmailMessage {
            to: "a@t.dev".to_owned(),
            template_key: "welcome".to_owned(),
            vars: serde_json::json!({}),
        };
        repo.enqueue(&msg).await.unwrap();
        repo.jobs.lock().unwrap()[0].scheduled_at = Utc::now() - Duration::minutes(6);
        // A row mid-'sending' whose scheduled_at is >= 5 min old is a crash
        // leftover: the second claim arm recovers it.
        let stale = repo.claim(Utc::now(), 1).await.unwrap().remove(0);
        let got = repo.claim(Utc::now(), 1).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, stale.id);
        // Inside the lease window (scheduled_at < 5 min old) it stays locked.
        repo.jobs.lock().unwrap()[0].scheduled_at = Utc::now() - Duration::minutes(4);
        repo.jobs.lock().unwrap()[0].status = "sending".to_owned();
        assert!(repo.claim(Utc::now(), 1).await.unwrap().is_empty());
    }
}

#[cfg(test)]
#[tokio::test]
async fn fail_backoff_dead_letter_and_complete() {
    let repo = MockOutboxRepository::new();
    let msg = EmailMessage {
        to: "a@t.dev".to_owned(),
        template_key: "welcome".to_owned(),
        vars: serde_json::json!({}),
    };
    let job = {
        repo.enqueue(&msg).await.unwrap();
        repo.claim(Utc::now() + Duration::hours(1), 1)
            .await
            .unwrap()
            .remove(0)
    };
    assert_eq!(job.status, "sending");
    // Fail #1: retry_count 1, still queued, scheduled ~2 (2^1) minutes out.
    repo.fail(&job, "smtp 5xx").await.unwrap();
    let stored = repo.jobs.lock().unwrap()[0].clone();
    assert_eq!(stored.status, "queued");
    assert_eq!(stored.retry_count, 1);
    let delta = (stored.scheduled_at - Utc::now()).num_seconds();
    assert!(
        delta > 90 && delta < 150,
        "2^1-minute backoff ±10% jitter, got {delta}s"
    );
    // complete: sent + sent_at + last_error cleared.
    repo.complete(job.id).await.unwrap();
    let stored = repo.jobs.lock().unwrap()[0].clone();
    assert_eq!(stored.status, "sent");
    assert!(stored.sent_at.is_some());
    assert!(stored.last_error.is_none());
    // Dead-letter: n >= max_retries → status 'failed' (no more retries).
    repo.enqueue(&msg).await.unwrap();
    let mut job2 = repo
        .claim(Utc::now() + Duration::hours(1), 1)
        .await
        .unwrap()
        .remove(0);
    job2.retry_count = 2; // n = retry_count + 1 = 3 >= max_retries 3
    repo.fail(&job2, "still down").await.unwrap();
    let stored = repo.jobs.lock().unwrap()[1].clone();
    assert_eq!(stored.status, "failed");
    assert_eq!(stored.retry_count, 3);
}
