# PLAYBOOK_BACKEND.md — backend architecture contract (Node.js)

The rules that keep `<app>` hexagonal and vertically sliced, written against
the code that actually ships: one ESM package, Fastify 5, `pg` + Kysely,
node-pg-migrate, an email outbox, an in-process realtime hub.

Premise for actors, roles, tables and invariants: `MEMORY.md`. This file is
the deep contract; `AGENTS.md` is the short one. When they disagree with the
code, the code wins and this file gets fixed in the same PR.

---

## 1. The model in one screen

| Layer            | Directory                       | Knows about                    | Never imports                      |
| ---------------- | ------------------------------- | ------------------------------ | ---------------------------------- |
| Driving adapters | `src/features/<slice>/http.ts`  | Fastify, zod, own use case     | other slices, `postgres.ts`        |
| Use case         | `usecase.ts`                    | `domain.ts`, `ports.ts`        | Fastify, pg, Kysely, zod, platform |
| Ports            | `ports.ts`                      | `domain.ts`                    | adapters                           |
| Domain           | `domain.ts`                     | Node stdlib, `#shared/*`       | everything else — **zero I/O**     |
| Driven adapters  | `postgres.ts`, `src/platform/*` | pg/Kysely or the outside world | other slices                       |
| Wiring           | `module.ts`                     | the slice's adapter + `Deps`   | —                                  |
| Composition root | `src/bin/*`                     | every slice and adapter        | — (nothing imports it)             |

Dependency direction is enforced by `no-restricted-imports` groups in
`eslint.config.mjs`, not by the module system. ESM happily loads cycles; the
linter is the law.

The three non-negotiable properties:

1. **The domain is pure.** `domain.ts` has types, invariants and error values,
   no await, no client.
2. **Ports are consumer-defined.** The use case writes the interface it needs,
   narrow (1–3 methods), in its own `ports.ts`. The adapter satisfies it
   structurally.
3. **Only the root wires concretions.** A slice receives `Deps` and returns
   nothing else. `test/acceptance/app.ts` is the second wiring and the proof.

---

## 2. Slice roles and the allowed edges

```text
src/bin/api.ts ──► every slice register() ──► Deps (concrete adapters)
      │                                              │
      └──► src/platform/* ◄── slices may import platform
                                     │
                              src/shared/* (kernel)
```

Rules, exact:

| From                                                       | May import                                                                |
| ---------------------------------------------------------- | ------------------------------------------------------------------------- |
| `src/bin/**`                                               | anything                                                                  |
| `src/features/<slice>/http.ts`, `postgres.ts`, `module.ts` | own slice, `#platform/*`, `#shared/*`                                     |
| `usecase.ts`                                               | `./domain.js`, `./ports.js`                                               |
| `ports.ts`                                                 | `./domain.js` (+ platform types where the port is platform-shaped)        |
| `domain.ts`                                                | Node stdlib, `#shared/*`                                                  |
| `src/platform/*`                                           | `#shared/*`, stdlib, `pg`, its own submodules — never `#features/*`       |
| `src/shared/*`                                             | stdlib only                                                               |
| anything                                                   | `src/bin/**` — forbidden                                                  |
| any slice                                                  | another slice — forbidden; `src/features/**` may not import `#features/*` |

Own-slice imports are relative (`./domain.js` — the `.js` extension is
required in ESM). Cross-layer imports use the package `"imports"` map:
`#shared/*`, `#platform/*`, `#features/*`, `#gen/*`.

Need another slice's behavior? Three lawful moves, in order:

1. **Promote down** to `#shared` (pure rule) or `src/platform` (infrastructure
   with no domain).
2. **Declare a port** in the consumer and satisfy it from the root with the
   other slice's adapter — the consumer never sees the producer's module.
3. **Duplicate the small capability** in the consumer when the two copies are
   genuinely independent (the signup and password slices both enqueue mail;
   neither imports the other).

Never add an eslint exception. A lint failure here is an architecture
decision made in private; make it in the open instead.

---

## 3. The slice shape

Every slice has the same eight files. "Where does this live?" is answered by
file kind, never by search.

```text
src/features/<slice>/
├── domain.ts        # pure types, invariants, AppError values
├── ports.ts         # interfaces the use case declares
├── usecase.ts       # orchestration: rules first, I/O last
├── queries.ts       # typed Kysely queries — the only SQL home
├── postgres.ts      # Store adapter over queries.ts
├── http.ts          # Fastify handlers, zod decode, envelope
├── module.ts        # register*(app, Deps)
└── usecase.test.ts  # fakes beside the code (when the slice has rules)
```

### 3.1 Domain

```ts
// src/features/widgets/domain.ts
export const WidgetKinds = ["sprocket", "cog", "flange"] as const;
export type WidgetKind = (typeof WidgetKinds)[number];

export interface Widget {
  id: string;
  name: string;
  kind: WidgetKind;
  ownerUserId: string;
  createdAt: Date;
}

export const ErrNotFound: AppError = E(CodeNotFound, NotFound, "widget not found");
export const ErrValidation: AppError = E(CodeUnprocessable, Validation, "invalid request");
```

- No `import` from Fastify/pg/Kysely/zod/platform/another slice.
- Invalid states are unrepresentable: unions over boolean pairs, literal
  unions for closed sets, `readonly` for data a callee must not mutate.
- Errors are values created by `E(http, code, detail)` from `#shared/apperr`.
  Never throw a bare string; never invent a code inline in `http.ts`.

### 3.2 Ports

```ts
// ports.ts — written for the use case, not for the database
export interface Store {
  insert(w: Widget): Promise<void>;
  list(ownerId: string, limit: number, offset: number): Promise<Widget[]>;
  get(id: string, ownerId: string): Promise<Widget | null>;
}
```

- Keep it to the methods this use case calls. A five-method port with one
  production implementation is usually two features sharing a directory.
- Methods return domain values or throw an `AppError`; adapters translate
  driver failures inward (`23505` → `AUTH_EMAIL_TAKEN` is the reference).
- No `Pool`, no `Kysely`, no `FastifyRequest` in a port signature.

### 3.3 Use case

```ts
import { systemClock, type Clock } from "#shared/clockx.js";
import { uuidGen } from "#shared/idgen.js";
import { roomUser, type RealtimeEvent } from "#platform/realtime/index.js";

export class UseCase {
  constructor(
    private readonly store: Store,
    private readonly pub: Publisher | null,
    private readonly clock: Clock = systemClock,
  ) {}

  async create(ownerId: string, name: string, kind: WidgetKind): Promise<Widget> {
    if (name === "") throw ErrValidation;
    const w: Widget = {
      id: uuidGen.newId(),
      name,
      kind,
      ownerUserId: ownerId,
      createdAt: this.clock.now(),
    };
    await this.store.insert(w);
    this.pub?.publish({
      room: roomUser(ownerId),
      type: "widget.created",
      payload: w,
      at: new Date(),
    });
    return w;
  }
}
```

- Validate the domain's rules here, once. Edge validation (shape, types,
  lengths) lives in `http.ts` with zod; the use case validates meaning.
- I/O last: compute and check first, then call the port.
- `AbortSignal` enters here when the work is long or batched; check
  `signal.throwIfAborted()` at loop yields, not after every await.
- No `console.*`, no `Date.now()` — take a clock (`#shared/clockx`) when time
  matters to the rule.

### 3.4 HTTP adapter

```ts
export const widgetHttp = (uc: UseCase): Handlers => ({
  create: async (req, reply) => {
    const body = decode(CreateBody, req.body); // 422 VALIDATION_ERROR
    const w = await uc.create(identityOf(req).userId, body.name, body.kind);
    await sendJSON(reply, 201, w);
  },
});
```

- `httpx.decode(schema, value)` is the only body decoder; it throws
  `VALIDATION_ERROR` on failure.
- `reply.code(...).send(...)` for success; errors are thrown and rendered once
  by the root's `setErrorHandler`. Do not hand-build error JSON here.
- Identity comes from `requireAuth`, which attaches `req.identity`; a small
  local `identityOf(req)` helper reads it. Never parse the JWT in a handler.
- One handler = one job. Mapping loops and multi-step orchestration belong in
  the use case.

### 3.5 Queries and the Store adapter

`queries.ts` is the only place SQL exists. Kysely builds it; column names stay
snake_case exactly as the migrations declare them.

```ts
export const widgetListByOwner = (
  db: Kysely<Database>,
  p: { ownerId: string; limit: number; offset: number },
): Promise<WidgetsRow[]> =>
  db
    .selectFrom("widgets")
    .selectAll()
    .where("owner_user_id", "=", p.ownerId)
    .orderBy("created_at", "desc")
    .limit(p.limit)
    .offset(p.offset)
    .execute();
```

- Export named query functions, one per statement, with the port method in
  the comment (`:one`, `:many`, `:exec`). No query builders leak out.
- The slice-local `Database` interface lists only the tables this slice reads
  or writes. Column types mirror the migration (`snake_case`).
- `postgres.ts` owns the `Kysely<Database>` instance built from the shared
  Pool; it implements `Store`, maps rows to domain values, and translates
  driver errors. Nothing else may import `queries.ts`.
- Two statements that must agree belong in one `db.transaction()` inside the
  adapter, not in the use case. The port should say so (`insertWithProfile`)
  instead of exposing a transaction handle.

### 3.6 Module

```ts
export interface Deps {
  store: Store;
  publisher: Publisher | null;
  verifier: Verifier;
  cookieName: string;
}

export const registerWidgets = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.publisher);
  const h = widgetHttp(uc, d.publisher);
  const auth = requireAuth(d.verifier, d.cookieName);
  app.get("/api/v1/widgets", { preHandler: [auth] }, h.list);
  // …
};
```

`register(app, Deps)` is the whole seam. It may construct the use case and
handlers; it may not open a pool, read env, or import another slice.

---

## 4. Composition root

`src/bin/api.ts` is one ordered function:

1. `load()` config (zod-validated, fail-closed).
2. Probe mode short-circuit (`dist/bin/api.js probe` → exit 0) before any I/O.
3. Fastify with `{ logger: { level: cfg.logLevel }, bodyLimit }`; cookie
   plugin; `onRequest` hooks `requestID`, `securityHeaders`, `cors(cfg)`.
4. `setNotFoundHandler` + `setErrorHandler` writing `{detail, code}`.
5. One `pg.Pool` (`newPool(cfg)`), one `Kysely` per _slice_ built inside its
   `Postgres` adapter.
6. Verifier + minter (`issuer = "<app>"`), cookie name/mode, the hub
   (`newHub(0)` → default buffer), the mail `Sender` (`selectSender`).
7. `/healthz` and `/readyz`.
8. Every slice's `register` with constructed `Deps`.
9. The in-process outbox worker on its own `AbortController`.
10. `listen(parseBind(cfg.bind))`; SIGTERM/SIGINT → abort worker,
    `app.close()`, `pool.end()`.

`src/bin/worker.ts` is the scale-out entry on the same image: the same pool,
one `AbortController`, the outbox loop **and** the janitor loop
(`sessions`, `email_verifications`, `password_resets` expired-row purges),
then `pool.end()`.

Both roots validate config at boot. Neither applies migrations.

Rules:

- Adapters are constructed at the root or in `module.ts`, never imported by a
  use case.
- One pool per process. Additional Kysely instances per slice are cheap
  views over it; additional pools are not.
- Shutdown is explicit: abort loops, close HTTP, end the pool. A dangling
  listener after SIGTERM is a defect.
- The root may know every slice; nothing else may know two.

---

## 5. Error model

Every non-2xx response has exactly one shape:

```json
{ "detail": "widget not found", "code": "NOT_FOUND" }
```

`AppError` (`#shared/apperr`) carries `http`, `code`, `detail`. Construction:

- `E(status, code, detail)` — the only constructor.
- `withDetail(e, d)` — same code, richer context for the boundary.
- `from(err)` — the single coercion from an unknown thrown value; anything
  unrecognized becomes `INTERNAL_ERROR` with a generic detail.

Rules:

- Stable codes are constants in `src/shared/apperr.ts`. Add a code there
  before using it; never type a string literal in a handler.
- Map HTTP status once, in the adapter that knows the failure. `401` for an
  invalid credential, `403` for an authenticated-but-forbidden action, `404`
  for a row the caller may not see (never leak existence with `403`), `409`
  for uniqueness, `422` for shape validation.
- 5xx detail is generic. Do not echo driver messages, SQL, or stack data.
- The root's `setErrorHandler` and a slice's `http.ts` must not both render
  errors. Handlers throw; the root renders. The only exception is a stream
  that already hijacked the reply — then close the stream.
- Adapters translate exactly once. A `23505` mapped in two places will drift.
- `catch` never swallows: log through `req.log` when the failure is not
  rethrown, and always with the stable code and request id.

---

## 6. Persistence

- **Postgres 17**, one `pg.Pool` at the root (`DATABASE_MAX_CONNECTIONS`,
  default 20).
- **Kysely per slice**, constructed in `postgres.ts` from that Pool. The
  `Database` type is slice-local and lists only owned tables.
- **SQL lives in `queries.ts`** and is consumed only by `postgres.ts`. No raw
  SQL in `usecase.ts` or `http.ts`; no query builder in a port signature.
- **Migrations live in `migrations/`** as timestamped `.mjs` files created by
  `make migrate-add NAME=…`. Every file exports `up` and `down`; an empty
  `down` is a review block.
- **Migrations never run at startup.** Apply with `make migrate-up`, or the
  compose one-shot `migrations` service. Replicas would race.
- **Naming**: snake_case tables and columns, plural tables, `id` text UUIDs,
  `created_at timestamptz` defaulted by the DB, partial indexes for
  active-row queries (`idx_sessions_user_active`).
- **No `SELECT *` into a port type**: select the columns the port promises,
  map explicitly. A schema addition should not silently widen a response.
- **Pagination**: lists take `limit` (1–100, default 20) and `offset` (≥0)
  through `httpx.queryPage`. An unbounded `execute()` on a user-sized table
  is a defect.
- **Ownership predicates are in the SQL**, not bolted on after the read:
  `get(id, ownerId)` filters both. An unowned row then reads as `null` → 404.
- **Destructive changes** (drop/rename column, backfill) get their own
  migration with a data plan in the file header. Never edit a migration that
  has shipped; add the next number.

The shipped schema (`0001_init`, `0002_profiles`): `users`, `sessions`,
`email_verifications`, `password_resets`, `settings`, `audit_logs`,
`email_jobs`, `widgets`, `companies`, `users_profile`.

---

## 7. Mail and the outbox

Slices never send mail. They enqueue intent; a worker renders and delivers.

```ts
export interface Message {
  to: string;
  templateKey: string;
  vars: Record<string, unknown>;
}
```

- The slice declares an `EmailEnqueuer` port (`enqueue(m)`); `platform/email`
  satisfies it. The use case calls it after the state change.
- `Outbox.enqueue` inserts into `email_jobs` (`id`, `to_email`,
  `template_key`, `payload`). Nothing is rendered on the request path.
- The worker claims due rows with
  `FOR UPDATE SKIP LOCKED` — safe with many replicas — flips them to
  `sending`, and recovers rows stranded there past a 5-minute lease
  (at-least-once).
- Delivery success → `sent`. Failure → `retry_count + 1` with a `2^n`-minute
  backoff plus ±10% jitter; at `max_retries` the row dead-letters as
  `failed`. Render errors fail immediately.
- Templates are keyed (`verify_email`, `reset_password`, `password_changed`,
  `welcome`) and localized by the user's `language` (`en`, `es`); an unknown
  language falls back to `en`.
- **Atomicity gap (shipped):** `enqueue` uses its own pool connection, so the
  mail row is not in the same transaction as the state change. A crash between
  them loses the mail. Do not describe the outbox as transactional until a
  transaction-capable port exists; the upgrade belongs in `Deps`, not in the
  use case.
- The janitor purges expired sessions and stale codes on
  `<APP>_JANITOR_POLL` (default 1h). It does not purge `email_jobs`; add a
  retention purge deliberately.

---

## 8. Realtime

One process-wide `EventBus` (`src/platform/realtime/hub.ts`), created at the
root and injected as each slice's `Publisher` port.

```ts
export interface RealtimeEvent {
  room: string; // "user:<id>" or a topic room
  type: string; // "widget.created"
  payload: unknown;
  at: Date;
}
export interface EventBus {
  publish(ev: RealtimeEvent): void;
  subscribe(room: string): Subscriber; // AsyncIterable + cancel()
}
```

- `publish` never blocks and never queues: when a subscriber's ring buffer
  (default 16) is full, the event is **dropped**. Slow consumers lose data;
  they do not apply backpressure to a request.
- `cancel()` is idempotent and removes the subscription; the iterator returns
  `done` on cancel.
- Room vocabulary is shared by WS and SSE: `roomUser(userId)` → `user:<id>`.
  A slice publishes to that room; it never imports the realtime slice.
- **WebSocket**: upgrade at `/api/v1/ws?token=<access-jwt>`. The browser
  cannot send an Authorization header on the handshake, so the token is
  verified _after_ the upgrade; failure closes with code `4001` and reason
  `authentication_failed`. The connection then only consumes its user room,
  pings every 30 s, and closes cleanly with `1000`.
- **SSE**: `GET /api/v1/widgets/events`, `text/event-stream`,
  `retry: 2000`, `event: <type>` + `data: <json>`. The handler hijacks the
  reply, subscribes to the user's room, and cancels on response `close`.
- A stream delivers only what the same identity could read over REST. A
  stream is not an authorization bypass.
- **Single-process by design.** Fan-out stops at the process boundary. The
  documented evolution is a Redis pub/sub adapter behind `EventBus`; the
  contract above is what must not change.

---

## 9. Workers and background work

Two backgrounds exist: the email outbox and the janitor.

Rules for any new loop:

- It has an owner: one `AbortController` per process, aborted on SIGTERM and
  SIGINT. No `setInterval` that outlives its owner.
- It is idempotent or claims rows with `SKIP LOCKED`. Multiple replicas must
  be safe; at-least-once is the delivery guarantee.
- It never throws out of the loop. A failed tick logs and continues; a crash
  loop needs an alert, not silent retries.
- It honors the signal during sleep, not after the sleep.
- It uses `pool.query` directly only for janitor-style bulk deletes; feature
  writes go through a slice's adapter.
- `console.*` is acceptable in `src/bin/worker.ts` for the loop's own
  lifecycle lines. Request paths use the Fastify/Pino logger.

Adding a queue? The outbox table is the pattern: durable row, lease,
backoff, dead-letter. Do not add BullMQ/Redis for a second queue without
writing the swap decision down — the adapter is replaceable, the contract is
not.

---

## 10. Configuration

`src/platform/config.ts`: one zod schema over `<APP>_`-prefixed env, parsed
once at boot. Unknown or malformed values fail the process — no silent
defaults for security-relevant keys.

- Every key is prefixed (`<APP>_DATABASE_URL`, `_JWT_SECRET`,
  `_PASSWORD_PEPPER`, `_ACCESS_TOKEN_TTL`, `_REFRESH_TOKEN_TTL`,
  `_VERIFY_CODE_TTL`, `_RESET_CODE_TTL`, `_RESET_RATE_LIMIT`,
  `_RESET_RATE_WINDOW`, `_MAX_LOGIN_RETRIES`, `_AUTH_COOKIE`,
  `_LOGOUT_ALL_ON_RESET`, `_CORS_ORIGINS`, `_GOOGLE_*`, `_GITHUB_*`,
  `_AUTH_GITHUB_CALLBACK_URL`, `_AUTH_FRONTEND_SUCCESS`, `_JANITOR_POLL`,
  `_ALLOWED_ADMIN_EMAILS`, `_SMTP_*`, `_OFFLINE`, `_OUTBOX_POLL`,
  `_OUTBOX_BATCH`, `_LOG_LEVEL`, `_BIND`, `_DATABASE_MAX_CONNECTIONS`).
- Durations accept `ms`/`s`/`m`/`h`; booleans are the literal `true`/`false`;
  lists are comma-separated.
- **Fail closed**: `JWT_SECRET` must be ≥32 bytes and not a placeholder;
  `PASSWORD_PEPPER`, when set, must be ≥32 bytes.
- **Offline guard (INV-T7)**: `<APP>_OFFLINE=true` forbids SMTP credentials
  at boot, forces the console sender, and makes OAuth flows refuse instead of
  opening sockets. A fresh `.env` with dummy values plus `OFFLINE=true` must
  boot a working stack.
- Config is read in the roots and passed down. A slice that calls
  `load()` or reads `process.env` is a defect; add a field to `Deps`.

---

## 11. Observability

- **Logs**: Fastify's Pino logger (JSON). Request paths use `req.log`, with
  `x-request-id` on every response. Never log passwords, tokens, cookies,
  authorization headers, or the WS query string.
- **Health**: `/healthz` is process liveness (no I/O). `/readyz` runs
  `SELECT 1` against the pool and reports failure as non-2xx.
- **Envelope**: stable `code` is the machine field; `detail` is for humans.
  Counters and alerts should key on `code`, not on message text.
- **Metrics/tracing are not shipped.** The seams are the Fastify hooks and
  the platform adapters; add OpenTelemetry there, not in every slice.
- **Log levels**: `debug` in development, `info` in production. A per-request
  log line at `info` on a hot path is a cost decision.

---

## 12. Deployment

- One Docker image (`node:24-alpine`, non-root `node`, `HEALTHCHECK` via
  `dist/bin/api.js probe`), two entrypoints: `dist/bin/api.js` and
  `dist/bin/worker.js`.
- `docker compose up --build` boots Postgres, a one-shot `migrations`
  service, the API, and the worker. The API never migrates itself.
- Single-replica deployments can run the outbox in-process (the API starts
  one) but still need the worker for the janitor.
- Scale-out is safe for outbox claims and janitor purges; realtime fan-out is
  not (see §8).
- No lockfile ships with the template (INV-T3). Decide deliberately after
  instantiation: commit `pnpm-lock.yaml` and remove the ignore entry if you
  need reproducible installs.

---

## 13. When to add a slice

Add a slice when a capability has its own rules, its own tables or its own
route group. Do not add one for a single handler over an existing slice.

The recipe is `docs/12-playbooks.md` §1; the proof it was done right:

- `make ci` green and `make accept` green.
- `git status` shows edits only under `src/features/<slug>/`, the root wiring
  (`src/bin/api.ts`), a new migration, and optionally a spec — nothing else.
- Deleting the slice directory plus its root registration leaves a compiling
  project (apart from the example's spec).

If adding a slice forced edits in three unrelated directories, the boundary
is wrong; fix the shape before landing it.

---

## 14. Checklist

- [ ] `domain.ts` imports only Node stdlib and `#shared/*`.
- [ ] Ports are consumer-defined, ≤3 methods, no driver types.
- [ ] SQL appears only in `queries.ts`; the adapter is the only caller.
- [ ] Errors are `AppError` values with codes from `src/shared/apperr`.
- [ ] The handler throws; the root renders; nothing double-renders.
- [ ] Lists are paginated with a hard maximum.
- [ ] Ownership is a predicate in the SQL, not a post-read check.
- [ ] A new table has a paired up/down migration and no startup migration.
- [ ] Mail goes through the outbox port, never an inline sender.
- [ ] Realtime publishes to a room; slices never import each other.
- [ ] New loops take the process signal and are replica-safe.
- [ ] New config is `<APP>_`-prefixed, zod-validated, fail-closed, documented
      in `.env.example`.
- [ ] Nothing new imports `src/bin/**`; nothing new imports another slice.

---

## 15. Known gaps in the shipped code

Not features. Fix deliberately or leave documented.

- No integration tier: adapter SQL and migration up/down are unproven by CI
  (`docs/11-testing-and-ci.md` §3).
- The outbox insert is not atomic with the state change (§7).
- `authsignup`, `authsession`, `authpassword`, `authoauth` and `realtime`
  have no unit tests; cucumber covers their behavior.
- Realtime is single-process (Redis pub/sub is the evolution).
- No coverage threshold, mutation testing, secret scan, SBOM or image scan.
- `POST /api/v1/auth/upgrade-to-admin` is unauthenticated and gated only by
  `ALLOWED_ADMIN_EMAILS`; remove it after bootstrap.
- The stack deltas vs beautifulnodejs's published digest (Kysely not Drizzle,
  DB outbox not BullMQ/Redis, constructor DI not tsyringe) are intentional.
  Swap an adapter; do not change the contract.
