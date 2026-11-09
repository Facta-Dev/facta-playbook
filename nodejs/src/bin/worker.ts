// Command worker runs the maintenance loops as their own entry point (same
// Docker image, different entry) for scale-out deployments: the email outbox
// claims with FOR UPDATE SKIP LOCKED, so many replicas are safe, and the
// janitor's idempotent purges are equally replica-safe. In single-binary
// deployments bin/api.ts already runs one outbox worker in-process.

import type { Pool } from "pg";
import { load } from "#platform/config.js";
import { newPool } from "#platform/pg.js";
import { Outbox, selectSender, runEmailWorker } from "#platform/email/index.js";

const main = async (): Promise<void> => {
  const cfg = load();
  const ctl = new AbortController();
  process.on("SIGTERM", () => {
    ctl.abort();
  });
  process.on("SIGINT", () => {
    ctl.abort();
  });
  const pool = await newPool(cfg);
  console.info("email worker listening", { poll: cfg.outboxPollMs, batch: cfg.outboxSize });
  // Two loops on one signal: the outbox drain and the janitor purge
  // (cmd/worker/main.go parity). Neither races a shutdown: the signal aborts
  // both, workers exit through the abort path, and pool.end() follows.
  await Promise.all([
    runEmailWorker(
      new Outbox(pool),
      selectSender(cfg, { info: (o: object) => console.info(o) }),
      cfg.outboxPollMs,
      cfg.outboxSize,
      ctl.signal,
    ).catch((err: unknown) => {
      console.error("email worker loop failed", err);
      ctl.abort();
    }),
    purgeLoop(pool, cfg.janitorPollMs, ctl.signal),
  ]);
  await pool.end();
  console.info("worker stopped");
};

// janitor is the worker-side housekeeping loop — reference parity for the
// celery auth_session_* tasks (expired-session cleanup + stale-code purge;
// the anomaly aggregate lives at the admin endpoint, computed on demand).
// purges lists the delete statements run on every tick: only expired rows go
// (sessions past expiry still marked active; codes past their lifetime).
// Rotation/revocation history is preserved until it expires. Deliberately
// simple — switch the code tables to a used-at purge if they accumulate.
const purges = [
  ["sessions", "DELETE FROM sessions WHERE is_active AND expires_at < now()"],
  ["email_verifications", "DELETE FROM email_verifications WHERE expires_at < now()"],
  ["password_resets", "DELETE FROM password_resets WHERE expires_at < now()"],
] as const;

// purgeLoop ticks every janitorPollMs (default 1h); a failing table skips
// only itself so one bad statement never starves the others.
const purgeLoop = async (pool: Pool, tickMs: number, signal: AbortSignal): Promise<void> => {
  const tick = tickMs > 0 ? tickMs : 3_600_000;
  let timer: NodeJS.Timeout | undefined;
  const wait = (): Promise<"tick" | "abort"> =>
    new Promise((res) => {
      timer = setTimeout(() => res("tick"), tick);
      signal.addEventListener(
        "abort",
        () => {
          clearTimeout(timer);
          res("abort");
        },
        { once: true },
      );
    });
  while (!signal.aborted) {
    const ev = await wait();
    if (ev === "abort" || signal.aborted) return;
    for (const [label, sql] of purges) {
      try {
        const ct = await pool.query(sql);
        const purged = ct.rowCount ?? 0;
        if (purged > 0) console.info("janitor", { table: label, purged });
      } catch (err) {
        console.error("janitor", { table: label, err });
      }
    }
  }
  if (timer !== undefined) clearTimeout(timer);
};

main().catch((err) => {
  console.error("fatal", err);
  process.exit(1);
});
