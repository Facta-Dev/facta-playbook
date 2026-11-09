//! Background workers — the email outbox drain + janitor housekeeping loops.
//!
//! Go `cmd/worker` (main.go + janitor.go) parity for the queue half of
//! `internal/platform/email/queue.go` (`RunWorker`/`drain`): claim due rows
//! from the durable outbox (the port's claim CTE is `FOR UPDATE SKIP LOCKED`,
//! so many replicas stay safe), render the localized template, deliver, and
//! record the outcome with 2^n-minute retry backoff (the backoff itself lives
//! in the `PgOutboxRepository::fail` adapter, go `Outbox.Fail` parity). The
//! janitor is the worker-side housekeeping loop (celery `auth_session_*`
//! parity): idempotent purges of expired rows, independent per table.
//!
//! Both loops take a `watch` stop receiver (go `ctx.Done` parity) and also
//! end when the paired sender is dropped.

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use <app>_core::{EmailJob, Error, Result};
use <app>_mailer::{Mailer, render};
use <app>_ports::OutboxRepository;
use sqlx::PgPool;
use tokio::sync::watch;

/// Go `RunWorker`: a non-positive batch falls back to 16 rows per claim.
pub const DEFAULT_BATCH: usize = 16;
/// Go config default: `<APP>_OUTBOX_POLL=2s`.
pub const DEFAULT_POLL: Duration = Duration::from_secs(2);
/// Go config default / janitor clamp: `<APP>_JANITOR_POLL=1h`.
pub const DEFAULT_JANITOR_TICK: Duration = Duration::from_secs(3600);
/// Go config default (env layer): `<APP>_OUTBOX_BATCH=10`. The loop itself
/// clamps a non-positive batch to [`DEFAULT_BATCH`] (go `RunWorker` parity).
pub const OUTBOX_BATCH_DEFAULT: i64 = 10;

/// One drain pass (go `email.drain`): claim a batch, render, deliver,
/// record. Errors are logged and swallowed — the loop keeps draining.
pub async fn drain(outbox: &dyn OutboxRepository, sender: &dyn Mailer, batch: usize) {
    let batch = if batch == 0 { DEFAULT_BATCH } else { batch };
    let jobs: Vec<EmailJob> = match outbox
        .claim(Utc::now(), i64::try_from(batch).unwrap_or(i64::MAX))
        .await
    {
        Ok(jobs) => jobs,
        Err(err) => {
            tracing::error!(%err, "email.claim");
            return;
        }
    };
    for job in jobs {
        // go: `lang, _ := j.Vars["locale"].(string)` — a non-string means "".
        let lang = job
            .vars
            .get("locale")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        match render(&job.template_key, &job.vars, lang) {
            Err(err) => {
                record_failure(outbox, &job, &err.to_string()).await;
            }
            Ok((subject, body)) => {
                if let Err(err) = sender.send(&job.to, &subject, &body).await {
                    record_failure(outbox, &job, &err.to_string()).await;
                    continue;
                }
                if let Err(err) = outbox.complete(job.id).await {
                    // Logged; the job re-leases at worst (at-least-once).
                    tracing::error!(%err, job_id = %job.id, "email.complete");
                }
            }
        }
    }
}

/// Record a delivery failure; a failing write is logged, not fatal (go drain).
async fn record_failure(outbox: &dyn OutboxRepository, job: &EmailJob, cause: &str) {
    if let Err(err) = outbox.fail(job, cause).await {
        tracing::error!(%err, job_id = %job.id, "email.fail");
    }
}

/// Drain the outbox on a poll cadence until stopped (go `email.RunWorker`).
///
/// Drains immediately, then once per tick. A zero `poll` is a
/// misconfiguration: go `time.NewTicker` panics there — here a validation
/// error the entry point turns into a fatal exit.
///
/// # Errors
/// [`Error::Invalid`] when `poll` is zero (nothing was drained).
pub async fn run_outbox_worker(
    outbox: Arc<dyn OutboxRepository>,
    sender: Arc<dyn Mailer>,
    poll: Duration,
    batch: usize,
    stop: watch::Receiver<()>,
) -> Result<()> {
    if poll.is_zero() {
        return Err(Error::Invalid(format!("poll must be > 0, got {poll:?}")));
    }
    let batch = if batch == 0 { DEFAULT_BATCH } else { batch };
    let mut stop = stop;
    let mut ticker = tokio::time::interval(poll);
    ticker.tick().await; // consume tokio's immediate first tick (go: fire at +poll)
    loop {
        drain(outbox.as_ref(), sender.as_ref(), batch).await;
        tokio::select! {
            _ = ticker.tick() => {}
            _ = stop.changed() => return Ok(()),
        }
    }
}

/// Worker-side housekeeping purges (go `janitor.purges`): only expired rows
/// go — sessions past expiry still marked active; codes past their lifetime.
/// Rotation/revocation history is preserved until it expires. Deliberately
/// simple — switch the code tables to a used-at purge if they accumulate.
const PURGES: &[(&str, &str)] = &[
    (
        "sessions",
        "DELETE FROM sessions WHERE is_active AND expires_at < now()",
    ),
    (
        "email_verifications",
        "DELETE FROM email_verifications WHERE expires_at < now()",
    ),
    (
        "password_resets",
        "DELETE FROM password_resets WHERE expires_at < now()",
    ),
];

/// The worker-side housekeeping loop (go `janitor.Run`): purge every tick
/// until stopped. `tick <= 0` (zero) clamps to 1h — go `Run` parity. No purge
/// runs before the first tick.
pub async fn run_janitor(pool: &PgPool, tick: Duration, stop: watch::Receiver<()>) {
    let tick = if tick.is_zero() {
        DEFAULT_JANITOR_TICK
    } else {
        tick
    };
    let mut stop = stop;
    let mut ticker = tokio::time::interval(tick);
    ticker.tick().await; // consume tokio's immediate first tick: purge fires on ticks only
    loop {
        tokio::select! {
            _ = ticker.tick() => purge(pool).await,
            _ = stop.changed() => return,
        }
    }
}

/// Run the delete set; a failing table skips only itself so one bad statement
/// never starves the others (go `janitor.purge`).
async fn purge(pool: &PgPool) {
    for (label, sql) in PURGES {
        match sqlx::query(sql).execute(pool).await {
            Ok(result) => {
                let purged = result.rows_affected();
                if purged > 0 {
                    tracing::info!(table = *label, purged, "janitor");
                }
            }
            Err(err) => {
                tracing::error!(table = *label, %err, "janitor");
            }
        }
    }
}

/// Go `time.ParseDuration` subset ("2s", "1h", "1h30m", "500ms", "0.5h") in
/// seconds: units `ms|s|m|h` (the python-entry subset), optional fractional
/// parts, multi-component strings — and the bare `0` (go accepts exactly it).
/// `None` = invalid.
#[must_use]
pub fn parse_duration(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None; // go ParseDuration(""): an invalid, empty duration
    }
    if raw == "0" {
        return Some(0.0);
    }
    let mut total = 0.0;
    let mut rest = raw;
    while !rest.is_empty() {
        let num_end = rest
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(rest.len());
        if num_end == 0 {
            return None;
        }
        let num: f64 = rest[..num_end].parse().ok()?;
        rest = &rest[num_end..];
        let (secs, taken) = if rest.starts_with("ms") {
            (0.001_f64, 2)
        } else {
            match rest.chars().next() {
                Some('s') => (1.0, 1),
                Some('m') => (60.0, 1),
                Some('h') => (3600.0, 1),
                _ => return None,
            }
        };
        total += num * secs;
        rest = &rest[taken..];
    }
    Some(total)
}

/// Env seconds: plain number or go-style duration ("1h"); `None` = invalid
/// (the entry point exits 1, go config-load error parity). Empty/unset
/// yields the default.
#[must_use]
pub fn env_seconds(name: &str, default: f64) -> Option<f64> {
    let Ok(raw) = std::env::var(name) else {
        return Some(default);
    };
    if raw.is_empty() {
        return Some(default);
    }
    raw.parse::<f64>().ok().or_else(|| parse_duration(&raw))
}

/// Env integer; `None` = unset-or-empty yields... see [`env_seconds`] — an
/// unset/empty var yields the default, a non-integer value yields `None`
/// (entry point exits 1, python `int(raw)` raise parity).
#[must_use]
pub fn env_int(name: &str, default: i64) -> Option<i64> {
    let Ok(raw) = std::env::var(name) else {
        return Some(default);
    };
    if raw.is_empty() {
        return Some(default);
    }
    raw.parse::<i64>().ok()
}

/// Bool env over alias list, first set wins (python `_env_bool` parity):
/// truthy in `{"1", "true", "yes", "on"}`, anything else false. An unset
/// var falls through to the next alias; all unset => `false`.
#[must_use]
pub fn env_bool(names: &[&str]) -> bool {
    let raw = names
        .iter()
        .find_map(|n| std::env::var(n).ok())
        .unwrap_or_default();
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use chrono::Utc;
    use tokio::sync::watch;

    use super::{
        DEFAULT_BATCH, drain, env_bool, env_int, env_seconds, parse_duration, run_outbox_worker,
    };
    use <app>_core::{EmailJob, EmailMessage, Error};
    use <app>_mailer::Mailer;
    use <app>_ports::OutboxRepository;
    use sqlx::types::uuid;

    // --- in-crate doubles (cross-crate helpers are forbidden) --------------

    struct FakeOutbox {
        due: Mutex<Vec<EmailJob>>,
        claims: Arc<AtomicUsize>,
        break_claim: bool,
        break_complete: bool,
        completed: Mutex<Vec<uuid::Uuid>>,
        failures: Mutex<Vec<(uuid::Uuid, String)>>,
    }

    impl FakeOutbox {
        fn with_due(jobs: Vec<EmailJob>) -> Self {
            Self {
                due: Mutex::new(jobs),
                claims: Arc::new(AtomicUsize::new(0)),
                break_claim: false,
                break_complete: false,
                completed: Mutex::new(Vec::new()),
                failures: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl OutboxRepository for FakeOutbox {
        async fn enqueue(&self, _message: &EmailMessage) -> Result<(), Error> {
            Ok(())
        }

        async fn claim(
            &self,
            _now: chrono::DateTime<Utc>,
            limit: i64,
        ) -> Result<Vec<EmailJob>, Error> {
            self.claims.fetch_add(1, Ordering::Relaxed);
            if self.break_claim {
                return Err(Error::Internal("db down".into()));
            }
            let mut due = self.due.lock().expect("poison");
            let take = usize::try_from(limit.max(0))
                .unwrap_or(usize::MAX)
                .min(due.len());
            Ok(due.drain(..take).collect())
        }

        async fn complete(&self, job_id: uuid::Uuid) -> Result<(), Error> {
            if self.break_complete {
                return Err(Error::Internal("write race".into()));
            }
            self.completed.lock().expect("poison").push(job_id);
            Ok(())
        }

        async fn fail(&self, job: &EmailJob, cause: &str) -> Result<(), Error> {
            self.failures
                .lock()
                .expect("poison")
                .push((job.id, cause.to_string()));
            Ok(())
        }
    }

    struct RecordingSender {
        fail_first: Mutex<usize>,
        sent: Mutex<Vec<(String, String, String)>>,
    }

    impl RecordingSender {
        fn with(fail_first: usize) -> Self {
            Self {
                fail_first: Mutex::new(fail_first),
                sent: Mutex::new(Vec::new()),
            }
        }

        fn fail_first_is_due(&self) -> bool {
            let mut n = self.fail_first.lock().expect("poison");
            let due = *n > 0;
            if due {
                *n -= 1;
            }
            due
        }
    }

    #[async_trait::async_trait]
    impl Mailer for RecordingSender {
        async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), Error> {
            if self.fail_first_is_due() {
                return Err(Error::Upstream {
                    service: "smtp",
                    detail: "smtp down".into(),
                });
            }
            self.sent.lock().expect("poison").push((
                to.to_string(),
                subject.to_string(),
                body.to_string(),
            ));
            Ok(())
        }
    }

    // One place to fix if the core EmailJob field set shifts (go email_jobs
    // defaults: status 'queued', max_retries 3).
    fn job(to: &str, key: &str, vars: serde_json::Value) -> EmailJob {
        EmailJob {
            id: uuid::Uuid::now_v7(),
            to: to.to_string(),
            template_key: key.to_string(),
            vars,
            status: "queued".to_string(),
            retry_count: 0,
            max_retries: 3,
            scheduled_at: Utc::now(),
            sent_at: None,
            last_error: None,
        }
    }

    // --- drain --------------------------------------------------------------

    #[tokio::test]
    async fn drain_delivers_due_email_with_locale() {
        let outbox = FakeOutbox::with_due(vec![job(
            "ana@example.com",
            "verify_email",
            serde_json::json!({"first_name": "Ana", "code": "123456", "locale": "es"}),
        )]);
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        {
            let sent = sender.sent.lock().expect("poison");
            assert_eq!(sent.len(), 1);
            let (to, subject, body) = &sent[0];
            assert_eq!(to, "ana@example.com");
            assert!(subject.contains("Verifica"), "es template pick: {subject}");
            assert!(
                body.contains("123456") && body.contains("Ana"),
                "es body: {body}"
            );
            drop(sent);
        }
        assert_eq!(outbox.completed.lock().expect("poison").len(), 1);
        assert!(outbox.failures.lock().expect("poison").is_empty());
    }

    #[tokio::test]
    async fn drain_non_string_locale_falls_back_to_en() {
        // go: `lang, _ := j.Vars["locale"].(string)` — a non-string means "".
        let outbox = FakeOutbox::with_due(vec![job(
            "a@example.com",
            "verify_email",
            serde_json::json!({"locale": 5}),
        )]);
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert_eq!(
            sender.sent.lock().expect("poison")[0].1,
            "Verify your email"
        );
    }

    #[tokio::test]
    async fn drain_unknown_lang_falls_back_to_en() {
        let outbox = FakeOutbox::with_due(vec![job(
            "a@example.com",
            "verify_email",
            serde_json::json!({"locale": "fr"}),
        )]);
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert_eq!(
            sender.sent.lock().expect("poison")[0].1,
            "Verify your email"
        );
    }

    #[tokio::test]
    async fn drain_send_failure_records_the_failure() {
        let outbox = FakeOutbox::with_due(vec![job(
            "a@example.com",
            "verify_email",
            serde_json::json!({}),
        )]);
        let sender = RecordingSender::with(1);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert!(sender.sent.lock().expect("poison").is_empty());
        let cause = {
            let failures = outbox.failures.lock().expect("poison");
            assert_eq!(failures.len(), 1);
            failures[0].1.clone()
        };
        // Recorded cause is `Error::Upstream`'s Display ("upstream <service>: <detail>").
        assert!(cause.contains("smtp down"), "{cause}");
    }

    #[tokio::test]
    async fn drain_unknown_template_records_failure_not_crash() {
        let outbox =
            FakeOutbox::with_due(vec![job("a@example.com", "nope", serde_json::json!({}))]);
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert!(sender.sent.lock().expect("poison").is_empty());
        let failures = outbox.failures.lock().expect("poison");
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].1.contains("unknown template"),
            "{}",
            failures[0].1
        );
    }

    #[tokio::test]
    async fn drain_claim_failure_is_swallowed() {
        // go: log "email.claim" and return; nothing else runs.
        let mut outbox = FakeOutbox::with_due(Vec::new());
        outbox.break_claim = true;
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert_eq!(outbox.claims.load(Ordering::Relaxed), 1);
        assert!(sender.sent.lock().expect("poison").is_empty());
    }

    #[tokio::test]
    async fn drain_complete_failure_is_swallowed() {
        let mut outbox = FakeOutbox::with_due(vec![job(
            "a@example.com",
            "verify_email",
            serde_json::json!({}),
        )]);
        outbox.break_complete = true;
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, DEFAULT_BATCH).await;
        assert_eq!(sender.sent.lock().expect("poison").len(), 1);
    }

    #[tokio::test]
    async fn drain_batch_clamps_to_default_minimum() {
        let due = (0..20)
            .map(|n| {
                job(
                    &format!("a{n}@example.com"),
                    "verify_email",
                    serde_json::json!({}),
                )
            })
            .collect();
        let outbox = FakeOutbox::with_due(due);
        let sender = RecordingSender::with(0);
        drain(&outbox, &sender, 0).await; // <=0 => 16 (go)
        assert_eq!(sender.sent.lock().expect("poison").len(), DEFAULT_BATCH);
        assert_eq!(outbox.due.lock().expect("poison").len(), 4);
    }

    // --- worker loop + env parsing ------------------------------------------

    /// Minimal claim counter; the loop test only observes claim cadence.
    struct CountingOutbox(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl OutboxRepository for CountingOutbox {
        async fn enqueue(&self, _message: &EmailMessage) -> Result<(), Error> {
            Ok(())
        }
        async fn claim(
            &self,
            _now: chrono::DateTime<Utc>,
            _limit: i64,
        ) -> Result<Vec<EmailJob>, Error> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Vec::new())
        }
        async fn complete(&self, _job_id: uuid::Uuid) -> Result<(), Error> {
            Ok(())
        }
        async fn fail(&self, _job: &EmailJob, _cause: &str) -> Result<(), Error> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_outbox_worker_drains_immediately_then_per_tick() {
        let claims = Arc::new(AtomicUsize::new(0));
        let outbox = Arc::new(CountingOutbox(Arc::clone(&claims)));
        let sender: Arc<dyn Mailer> = Arc::new(RecordingSender::with(0));
        let (stop_tx, stop_rx) = watch::channel(());
        let task = tokio::spawn(run_outbox_worker(
            outbox,
            sender,
            std::time::Duration::from_millis(50),
            DEFAULT_BATCH,
            stop_rx,
        ));
        // One drain before the first tick + at least one ticked drain.
        for _ in 0..300 {
            if claims.load(Ordering::Relaxed) >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        drop(stop_tx); // go ctx.Done parity: dropping the sender stops the loop
        task.await.expect("worker task ends").expect("clean stop");
        assert!(claims.load(Ordering::Relaxed) >= 2);
    }

    #[tokio::test]
    async fn run_outbox_worker_requires_positive_poll() {
        let outbox = FakeOutbox::with_due(Vec::new());
        let sender = RecordingSender::with(0);
        let res = run_outbox_worker(
            Arc::new(outbox),
            Arc::new(sender),
            std::time::Duration::ZERO,
            DEFAULT_BATCH,
            watch::channel(()).1,
        )
        .await;
        assert!(matches!(res, Err(Error::Invalid(_))), "{res:?}");
    }

    #[test]
    fn parses_go_style_durations() {
        assert_eq!(parse_duration("2s"), Some(2.0));
        assert_eq!(parse_duration("1h"), Some(3600.0));
        assert_eq!(parse_duration("1h30m"), Some(5400.0));
        assert_eq!(parse_duration("500ms"), Some(0.5));
        assert_eq!(parse_duration("0.5h"), Some(1800.0));
        assert_eq!(parse_duration("0"), Some(0.0));
        assert_eq!(parse_duration("soon"), None);
        assert_eq!(
            parse_duration("5"),
            None,
            "bare numbers ride the float path"
        );
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("1x"), None);
    }

    #[test]
    fn env_unset_yields_default() {
        // The <APP>_ names are never present in a test process, so this pins
        // the unset => default branch; the set branches cannot be tested
        // without unsafe env mutation (workspace forbids unsafe). Pure parsing
        // is covered by `parses_go_style_durations`.
        assert_eq!(env_seconds("<APP>_OUTBOX_POLL", 2.0), Some(2.0));
        assert_eq!(env_int("<APP>_OUTBOX_BATCH", 10), Some(10));
        assert!(!env_bool(&["<APP>_OFFLINE", "OFFLINE"]));
    }
}
