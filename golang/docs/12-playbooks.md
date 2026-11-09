# Playbooks

Repeatable recipes for the five changes you will make most. Each: steps, the
files touched, and the gate that proves it. If a change touches more files
than listed, the architecture regressed — investigate before proceeding.

## 1. Add a slice (new feature)

1. `internal/features/<slugs>/` with the six files. Start from the domain:
   `domain.go` (types, sentinel errors with stable codes in `apperr`),
   `ports.go` (the 1–3 interfaces the use case needs), `usecase.go`.
2. Schema: `make migrate-add NAME=<slugs>_tables` → fill the Up section; write the
   slice's `queries.sql`; run `make generate`.
3. `postgres.go` — implement the port(s) over `gen/db`; `http.go` — routes +
   envelope responses; `module.go` — `Register(mux, Deps)`.

```go
// module.go
type Deps struct {
    Store ports.UserWriter
    Queue email.Queue
    Admin authx.Identity // for requireRole; wire real middleware at root
}
func Register(mux *http.ServeMux, d Deps) {
    mux.HandleFunc("POST /api/v1/<resource>", httpx.Chain(h.Create, d.mws...)...)
}
```

4. Wire at **`cmd/api/main.go` only**: construct the adapter(s), call
   `Register`. Add `specs/features/<slugs>.feature` for behavioral contract.
5. Gate: `make ci` + `make accept` (`make test-int` if new tables).

## 2. Add a port + adapter pair

1. In the consuming slice's `ports.go`, declare the interface (1–2 methods,
   consumer-named): `type Notifier interface { Notify(ctx, n Notification) error }`.
2. Implement the adapter where the mechanism lives: an existing
   `postgres.go`, a new `platform/<x>/` package, or an in-slice fake-backed
   stub with a `// ponytail:` note naming its ceiling.
3. Wire: the root constructs the adapter; the slice's `Deps` grows one field.
   For an alternate implementation (e.g. fake in tests), nothing else changes.

## 3. Ship a migration

```sh
make migrate-add NAME=add_orders_status   # stamps <ts>_add_orders_status.sql (Up + Down sections)
# edit both files (down = true reversal, not a comment)
make generate                              # refresh gen/db if queries change
make migrate-up                           # apply locally
go test -tags=integration ./internal/...
```

Rules that keep this boring: paired up/down always; no startup migrations —
apply in your deploy step; destructive changes (drop/change column) get their
own migration with a data-migration plan noted in the file header; the
template ships `0001_init` + `0002_profiles` as the reference — in your
project, edit your own 0001 before first deploy, never after.

## 3b. Run the worker with the API

`cmd/worker` is a second entrypoint on the same image: the email outbox loop
plus the janitor (expired sessions + stale codes, `<APP>_JANITOR_POLL`,
default 1h). Deploy step: run one worker alongside every API replica set —
the loops are idempotent and replica-safe, so one worker or several both
work, but a deployment with *no* worker quietly stops mail delivery and
table purges. Compose ships the second service; keep it when you compose your
own deploy.

## 4. Expose a new endpoint (existing slice)

1. `specs/openapi.yaml` (contract first) → regenerate `make generate` → the
   wire types appear in `internal/gen/api`.
2. `http.go`: handler + edge validation (`validator/v10`), envelope on error,
   auth posture (`RequireAuth` / `RequireRole`) chained at the mount.
3. `module.go`: add the `mux.HandleFunc("METHOD /api/v1/...", ...)` line.
4. Wire the middleware chain in `cmd/api/main.go` if a new posture group is
   needed; otherwise the existing chain applies.
5. Acceptance scenario + unit test; `make ci`.

## 5. Add an email template

1. `internal/platform/email/templates/`: key the file set
   `verify_email`, `reset_password`, `password_changed`… — one template per
   key, localized by the user's `language` field (`en`, `es` shipped).
2. Declare the template key as a typed constant next to the sender; the
   `Message.Vars` map carries interpolation data (`code`, `first_name`, ...).

```go
msg := email.Message{
    To: user.Email, TemplateKey: email.VerifyEmail,
    Vars: map[string]any{"code": code, "first_name": user.FirstName, "locale": user.Language},
}
```

3. Enqueue from the use case via the `MailQueue` port — never call a sender
   directly from a use case (delivery is the worker's job; retries and
   dead-lettering apply uniformly).
4. Template test (render + vars) is a unit test in the platform package; the
   outbox integration tag covers claim/retry semantics once, not per template.

## 6. (Bonus) Replace the widgets example

Delete `internal/features/widgets` (including its `queries.sql`) + the
`widgets` table from your 0001, **or** rename it in place as your first real
slice (its SSE publisher is the reference wiring for hub events). Either way,
`make ci` + `make accept` after the change — the godog spec
`specs/features/widgets.feature` goes with the slice.

## See also

- [Project Skeleton](13-project-skeleton.md)
- [Testing & CI](11-testing-and-ci.md)
- [Design Patterns](05-design-patterns.md)