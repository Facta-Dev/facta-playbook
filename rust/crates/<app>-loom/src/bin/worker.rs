//! Worker entry point (go `cmd/worker/main.go` parity): the email outbox
//! drain + the janitor purge loops run concurrently on one shutdown signal
//! (SIGINT/SIGTERM). Runs the same loops cmd/api would run in-process;
//! outbox claims are `FOR UPDATE SKIP LOCKED`, so replicas are safe.
//!
//! Layering: the bin reads `<APP>_`-prefixed env vars directly (go
//! `config.Load` env parity) instead of importing the api config crate —
//! a binary in `<app>-loom` must not depend on the composition root.

use std::{sync::Arc, time::Duration};

use <app>_loom::workers::{
    self, DEFAULT_JANITOR_TICK, DEFAULT_POLL, env_bool, env_int, env_seconds,
};
use <app>_mailer::select_sender;
use <app>_ports::adapters::PgOutboxRepository;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::watch;

fn main() {
    // go: slog default logger parity — env filter with a sane default.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|err| fatal(&format!("tokio runtime: {err}")));
    let code = rt.block_on(run());
    std::process::exit(code);
}

/// `slog.Error("fatal", "err", ...)` + `os.Exit(1)` parity.
fn fatal(err: &str) -> ! {
    tracing::error!("fatal err={err}");
    std::process::exit(1);
}

async fn run() -> i32 {
    // env-first config (no api config crate). go `Config.Load`: a missing
    // DATABASE_URL is a config error => exit 1.
    let database_url = std::env::var("<APP>_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| fatal("<APP>_DATABASE_URL is required"));

    // go `Config.Load` INV-T7 validation: offline forbids SMTP credentials.
    let offline = env_bool(&["<APP>_OFFLINE", "OFFLINE"]);
    let smtp_host = std::env::var("<APP>_SMTP_HOST").unwrap_or_default();
    let smtp_user = std::env::var("<APP>_SMTP_USER").unwrap_or_default();
    let smtp_pass = std::env::var("<APP>_SMTP_PASS").unwrap_or_default();
    if offline && (!smtp_host.is_empty() || !smtp_user.is_empty() || !smtp_pass.is_empty()) {
        fatal(
            "config: <APP>_OFFLINE=true forbids SMTP credentials (INV-T7); use the console sender",
        );
    }
    let smtp_port = env_int("<APP>_SMTP_PORT", 587).map_or_else(
        || fatal("<APP>_SMTP_PORT: invalid port"),
        |v| u16::try_from(v).unwrap_or_else(|_| fatal("<APP>_SMTP_PORT: invalid port")),
    );
    let smtp_from = std::env::var("<APP>_SMTP_FROM").unwrap_or_default();

    // go `email.SelectSender`: offline or blank host => console sender.
    let sender = select_sender(
        offline, &smtp_host, smtp_port, &smtp_from, &smtp_user, &smtp_pass,
    );

    // go pgx.NewPool: eager connect + ping; a failure is fatal (exit 1).
    let pool = match PgPoolOptions::new()
        .max_connections(20) // go DBMaxConns envDefault 20
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(err) => fatal(&format!("db connect: {err}")),
    };

    let outbox = Arc::new(PgOutboxRepository::new(pool.clone()));

    // Durations: plain seconds (float) or go-style strings ("1h30m"); an
    // invalid value is a config error => exit 1 (go env-parse parity). A zero
    // JANITOR_POLL clamps to 1h inside the loop (go `janitor.Run` parity); a
    // zero OUTBOX_POLL is rejected by the loop (go NewTicker-panic parity).
    let seconds = |name: &str, default: f64| -> Duration {
        match env_seconds(name, default) {
            Some(s) if s.is_finite() => Duration::try_from_secs_f64(s)
                .unwrap_or_else(|_| fatal(&format!("{name}: invalid duration"))),
            _ => fatal(&format!("{name}: invalid duration")),
        }
    };
    let poll_interval = seconds("<APP>_OUTBOX_POLL", DEFAULT_POLL.as_secs_f64());
    let batch = env_int("<APP>_OUTBOX_BATCH", workers::OUTBOX_BATCH_DEFAULT)
        .unwrap_or_else(|| fatal("<APP>_OUTBOX_BATCH: invalid batch"));
    let tick = seconds(
        "<APP>_JANITOR_POLL",
        DEFAULT_JANITOR_TICK.as_secs_f64(),
    );

    tracing::info!(poll = ?poll_interval, batch, "email worker listening");

    // Two loops, one shutdown signal; neither races the shutdown (go main).
    let (stop_tx, stop_rx) = watch::channel(());
    let janitor_rx = stop_tx.subscribe();
    let signal = tokio::spawn(shutdown_signal(stop_tx));
    let (worker, ()) = tokio::join!(
        workers::run_outbox_worker(
            outbox,
            sender,
            poll_interval,
            usize::try_from(batch).unwrap_or(usize::MAX),
            stop_rx,
        ),
        workers::run_janitor(&pool, tick, janitor_rx),
    );
    signal.abort();
    if let Err(err) = worker {
        fatal(&err.to_string());
    }
    tracing::info!("worker stopped");
    0
}

/// go `signal.NotifyContext(SIGINT, SIGTERM)` parity: fire the stop channel
/// on either signal.
async fn shutdown_signal(stop: watch::Sender<()>) {
    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap_or_else(|err| fatal(&format!("install SIGTERM handler: {err}")));
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        };
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    let _ = stop.send(()); // a dropped stop sender also stops the loops
}
