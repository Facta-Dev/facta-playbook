# Playbooks

Repeatable recipes for the five changes you will make most. Each: steps, the
files touched, and the gate that proves it. If a change touches more files
than listed, the architecture regressed — investigate before proceeding.

## 1. Add a slice (new feature)

1. `src/features/<slug>/` with the same file set as every other slice. Start
   from the domain: `domain.ts` (types, `AppError`s with stable codes in
   `apperr`), `ports.ts` (the 1–3 interfaces the use case needs),
   `usecase.ts`.
2. Schema: `make migrate-add NAME=<slug>_tables` → fill the `up`/`down`
   exports; write the slice's `queries.ts` (typed Kysely over the slice's
   `Database` map).
3. `postgres.ts` — implement the port(s) over the queries; `http.ts` —
   handlers + zod decode + envelope responses; `module.ts` —
   `register(app, Deps)`.

```ts
// module.ts
export interface Deps {
  store: ports.UserWriter;
  emails: EmailEnqueuer;
  clock: Clock;
  pepper: string;
}
export const registerUsers = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.emails, d.clock, d.pepper);
  const h = userHttp(uc);
  const auth = requireAuth(d.verifier, d.cookieName); // wired from root_deps
  app.post("/api/v1/<resource>", { preHandler: [auth] }, h.create);
};
```

4. Wire at **`src/bin/api.ts` only**: construct the adapter(s), call
   `register`. Add `specs/features/<slug>.feature` for behavioral contract.
5. Gate: `make ci` + `make accept`. If the slice shipped tables, exercise
   the new SQL against a real Postgres once by hand (`make migrate-up`, curl
   the endpoints) — the template ships no integration lane (§3).

## 2. Add a port + adapter pair

1. In the consuming slice's `ports.ts`, declare the interface (1–2 methods,
   consumer-named): `type Notifier = { notify(n: Notification): Promise<void> }`.
2. Implement the adapter where the mechanism lives: an existing
   `postgres.ts`, a new `platform/<x>/` module, or an in-slice fake-backed
   stub with a `// ponytail:` note naming its ceiling.
3. Wire: the root constructs the adapter; the slice's `Deps` grows one field.
   For an alternate implementation (e.g. fake in tests), nothing else changes.

## 3. Ship a migration

```sh
make migrate-add NAME=add_orders_status   # stamps <ts>_add_orders_status.mjs (up + down exports)
# edit both exports (down = true reversal, not a comment)
make migrate-up                           # apply locally
```

Rules that keep this boring: paired up/down always; no startup migrations —
the compose boot path runs the one-shot `migrations` service (or your deploy
step does the equivalent); destructive changes (drop/change column) get their
own migration with a data-migration plan noted in the file header; the
template ships `0001_init` + `0002_profiles` as the reference — in your
project, edit your own 0001 before first deploy, never after.

## 3b. Run the worker with the API

`bin/worker.ts` is a second entrypoint on the same image (`dist/bin/worker.js`
in compose): the email outbox loop plus the janitor (expired sessions + stale
codes, `<APP>_JANITOR_POLL`, default 1h). Single-replica deployments already
drain the outbox in-process (bin/api.ts starts one worker on the same
AbortController); the claims are `FOR UPDATE SKIP LOCKED`, so api + worker +
extra replicas are all safe together. What only `bin/worker.ts` does is the
janitor — a deployment with _no_ worker quietly stops table purges. Compose
ships the second service; keep it when you compose your own deploy.

## 4. Expose a new endpoint (existing slice)

1. `specs/openapi.yaml` (contract first) → regenerate `make generate` → the
   wire types appear in `src/gen/api`.
2. `http.ts`: handler + edge validation (a zod schema mirroring the contract,
   decoded with `httpx.decode`), envelope on error, auth posture
   (`requireAuth` / `requireRole`) chained via the `preHandler` array at the
   mount.
3. `module.ts`: add the `app.get/post/...("/api/v1/...", { preHandler: [...] },
h.x)` line.
4. Wire a new posture group in `bin/api.ts` only if genuinely needed —
   otherwise the existing hook chain (request-id, security headers, CORS)
   applies.
5. Acceptance scenario + unit test; `make ci`.

## 5. Add an email template

1. `platform/email/templates.ts`: key the entry `verify_email`,
   `reset_password`, `password_changed`… — one key per entry, localized by
   the user's `language` field (`en`, `es` shipped).
2. `{{var}}` placeholders interpolate from `Message.vars`; unknown languages
   fall back to `en` and unknown keys fail the delivery (render errors
   dead-letter through the queue, visibly):

```ts
await emails.enqueue({
  to: user.email,
  templateKey: "verify_email",
  vars: { code, first_name: user.firstName, locale: user.language },
});
```

3. Enqueue from the use case via the emails port — never call a `Sender`
   directly from a use case (delivery is the worker's job; retries and
   dead-lettering apply uniformly).
4. Template test (render + vars) is a unit test in the platform package; the
   outbox integration lane covers claim/retry semantics once, not per
   template.

## 6. (Bonus) Replace the widgets example

Delete `src/features/widgets` (including its `queries.ts`) + the `widgets`
table from your 0001, **or** rename it in place as your first real slice (its
SSE publisher is the reference wiring for hub events). Either way, `make ci`

- `make accept` after the change — the cucumber spec `specs/features/widgets.feature`
  goes with the slice.

## See also

- [Project Skeleton](13-project-skeleton.md)
- [Testing & CI](11-testing-and-ci.md)
- [Design Patterns](05-design-patterns.md)
