// The durable outbox: slices enqueue Messages into a Postgres table; a
// worker claims rows (SELECT ... FOR UPDATE SKIP LOCKED), renders the
// localized template and delivers. Port of the Go template's email/queue.go.

import crypto from "node:crypto";
import type { Pool } from "pg";
import type { Sender } from "./sender.js";
import { render } from "./templates.js";

// Message is what a slice wants delivered: template + vars, not rendered bytes.
export interface Message {
  to: string;
  templateKey: string;
  vars: MessageVars;
}

export type MessageVars = Record<string, unknown>;

// Job is a claimed outbox row inside the worker pipeline.
export interface Job extends Message {
  id: string;
  retryCount: number;
  maxRetries: number;
  scheduledAt: Date;
}

// Queue is the durable outbox contract (impl: Outbox below).
export interface Queue {
  enqueue(message: Message): Promise<void>;
  claim(now: Date, limit: number): Promise<Job[]>;
  complete(jobId: string): Promise<void>;
  // fail records the failure; the adapter retries with 2^n-minute backoff and
  // dead-letters (status 'failed') once retryCount >= maxRetries.
  fail(job: Job, cause: Error): Promise<void>;
}

// Outbox is the pg adapter over the email_jobs table.
export class Outbox implements Queue {
  constructor(private readonly pool: Pool) {}

  async enqueue(m: Message): Promise<void> {
    await this.pool.query(
      "INSERT INTO email_jobs (id, to_email, template_key, payload) VALUES ($1, $2, $3, $4)",
      [crypto.randomUUID(), m.to, m.templateKey, JSON.stringify(m.vars ?? {})],
    );
  }

  // claim locks up to `limit` due rows and flips them to 'sending'. The second
  // arm recovers rows stranded in 'sending' by a crashed worker: they become
  // claimable again after the 5-minute lease, at-least-once.
  async claim(now: Date, limit: number): Promise<Job[]> {
    const r = await this.pool.query(
      `WITH picked AS (
         SELECT id FROM email_jobs
         WHERE (status = 'queued' AND scheduled_at <= $1)
            OR (status = 'sending' AND scheduled_at <= $1 - interval '5 minutes')
         ORDER BY scheduled_at
         LIMIT $2
         FOR UPDATE SKIP LOCKED
       )
       UPDATE email_jobs j SET status = 'sending'
       WHERE j.id IN (SELECT id FROM picked)
       RETURNING j.id, j.to_email, j.template_key, j.payload, j.retry_count, j.max_retries, j.scheduled_at`,
      [now, limit],
    );
    return r.rows.map((row: Record<string, unknown>) => ({
      id: row["id"] as string,
      to: row["to_email"] as string,
      templateKey: row["template_key"] as string,
      vars: (row["payload"] ?? {}) as MessageVars,
      retryCount: row["retry_count"] as number,
      maxRetries: row["max_retries"] as number,
      scheduledAt: new Date(row["scheduled_at"] as string),
    }));
  }

  async complete(jobId: string): Promise<void> {
    await this.pool.query(
      "UPDATE email_jobs SET status = 'sent', sent_at = now(), last_error = NULL WHERE id = $1",
      [jobId],
    );
  }

  // fail bumps retry_count with 2^n-minute jittered backoff; at max_retries
  // the job dead-letters (status 'failed') instead of retrying forever.
  async fail(job: Job, cause: Error): Promise<void> {
    const n = job.retryCount + 1;
    const backoffMs = 2 ** n * 60_000;
    // Jitter ±20% OF THE BACKOFF (not the wall clock) so simultaneous
    // failures don't re-synchronize polling.
    const jitter = Math.floor(Math.random() * (backoffMs / 5)) - backoffMs / 10;
    const scheduled = new Date(Date.now() + backoffMs + jitter);
    const status = n >= job.maxRetries ? "failed" : "queued";
    await this.pool.query(
      "UPDATE email_jobs SET status = $2, retry_count = $3, last_error = $4, scheduled_at = $5 WHERE id = $1",
      [job.id, status, n, cause.message, scheduled],
    );
  }
}

// runEmailWorker drains the outbox until the abort signal fires: claim a
// batch (DB fetch size), render, deliver, then sleep out the poll interval.
export const runEmailWorker = async (
  q: Queue,
  s: Sender,
  pollMs: number,
  batch: number,
  signal: AbortSignal,
): Promise<void> => {
  const b = batch > 0 ? batch : 16;
  while (!signal.aborted) {
    await drain(q, s, b);
    await sleepMs(pollMs, signal);
  }
};

const sleepMs = (ms: number, signal: AbortSignal): Promise<void> =>
  new Promise((res) => {
    const t = setTimeout(res, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(t);
        res();
      },
      { once: true },
    );
  });

// drain claims one batch, renders and delivers; failures dead-letter via
// Queue.fail (backoff + retries), render errors fail immediately.
export const drain = async (q: Queue, s: Sender, batch: number): Promise<void> => {
  let jobs: Job[] = [];
  try {
    jobs = await q.claim(new Date(), batch);
  } catch {
    return; // transient claim trouble: try again after the poll interval
  }
  for (const j of jobs) {
    await deliverOne(q, s, j);
  }
};

const deliverOne = async (q: Queue, s: Sender, j: Job): Promise<void> => {
  const lang = typeof j.vars["locale"] === "string" ? (j.vars["locale"] as string) : "";
  const rendered = render(j.templateKey, j.vars, lang);
  if (!rendered.ok) {
    await q.fail(j, rendered.err);
    return;
  }
  try {
    await s.send(j.to, rendered.subject, rendered.body);
  } catch (err) {
    await q.fail(j, err instanceof Error ? err : new Error(String(err)));
    return;
  }
  await q.complete(j.id);
};
