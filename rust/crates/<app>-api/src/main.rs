use anyhow::Result;
use tracing::info;

use <app>_api::Config;
use <app>_api::telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    // Subcommand: `<app>-api openapi` prints the OpenAPI spec to stdout.
    // Used by `just gen-openapi` / `just gen-check`. No config or DB needed.
    if std::env::args().nth(1).as_deref() == Some("openapi") {
        let doc = <app>_api::router::openapi();
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }

    // Subcommand: `<app>-api migrate` — one-shot schema runner (go
    // cmd/migrate parity). The API never migrates itself (replicas race); the
    // compose `migrations` service runs this before api/worker boot, against
    // the DDL embedded at compile time by sqlx::migrate! (no live DB or
    // sqlx-cli needed at build).
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let database_url = std::env::var("<APP>_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .map_err(|_| anyhow::anyhow!("<APP>_DATABASE_URL is required"))?;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await?;
        sqlx::migrate!("../../migrations").run(&pool).await?;
        pool.close().await;
        return Ok(());
    }

    // Subcommand: `<app>-api healthcheck` — container HEALTHCHECK probe.
    // Used by Dockerfile.static (distroless/static has no shell/curl, so the
    // binary probes its own listener). TCP-connects to <APP>_BIND (default
    // 0.0.0.0:8080); exits 0 on connect, 1 on failure. No config/DB load —
    // only the bind address is read, so a misconfigured secret cannot mask a
    // liveness failure. Run by Docker's HEALTHCHECK directive in the runtime
    // image.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let bind = std::env::var("<APP>_BIND").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let addr: std::net::SocketAddr = bind
            .parse()
            .map_err(|e| anyhow::anyhow!("<APP>_BIND parse failed: {e}"))?;
        return tokio::net::TcpStream::connect(&addr)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("healthcheck: connect to {addr} failed: {e}"));
    }

    let cfg = Config::from_env()?;
    let _telemetry_guard = telemetry::init(&cfg)?;

    #[cfg(feature = "metrics")]
    if <app>_api::metrics::init(&cfg) {
        info!("prometheus metrics recorder installed (GET /metrics)");
    }

    info!(env = %cfg.env, bind = %cfg.bind, "starting <app>-api");

    let state = <app>_api::AppState::from_env(&cfg).await?;

    // The email outbox worker runs in-process (go cmd/api/main.go parity:
    // email.RunWorker on its own cancel, joined after the server exits).
    // The janitor is worker-binary territory (<APP>_JANITOR_POLL is used by
    // the `<app>-worker` binary, not here).
    let (worker_stop, worker_rx) = tokio::sync::watch::channel(());
    let poll = <app>_api::config::parse_go_duration(&cfg.outbox_poll)?;
    let worker = {
        let outbox = state.outbox.clone();
        let mailer = state.mailer.clone();
        let stop = worker_rx.clone();
        tokio::spawn(<app>_loom::workers::run_outbox_worker(
            outbox,
            mailer,
            poll,
            cfg.outbox_size,
            stop,
        ))
    };

    let app = <app>_api::router::build(state);
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    info!(addr = %listener.local_addr().map_or_else(|_| "?".into(), |a| a.to_string()), "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // go chain: srv.Shutdown → baseCancel → workerStop → wg.Wait. HTTP has
    // drained; NOW stop the outbox worker and join its final pass.
    let _ = worker_stop.send(());
    if let Ok(Err(err)) = worker.await {
        tracing::error!(%err, "outbox worker exited with error");
    }
    info!("stopped");
    Ok(())
}

/// SIGINT/SIGTERM select; resolving lets the server drain in-flight requests.
async fn shutdown_signal() {
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler installs");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => info!(signal = "SIGINT", "caught"),
        _ = sigterm.recv() => info!(signal = "SIGTERM", "caught"),
    }
}
