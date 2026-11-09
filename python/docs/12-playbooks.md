# Playbooks

Repeatable recipes for the five changes you will make most. Each: steps, the
files touched, and the gate that proves it. If a change touches more files
than listed, the architecture regressed — investigate before proceeding.

## 1. Add a slice (new feature)

1. `src/app/features/<slug>/` with the same file set as every other slice.
   Start from the domain: `domain.py` (types and invariants, zero I/O),
   `ports.py` (the 1–3 `Protocol`s the use case needs), `usecase.py`.
2. Schema: `make migrate-add NAME=<slug>_tables` → fill `upgrade()` and
   `downgrade()`; write the slice's `queries.py` (parameterized
   `sqlalchemy.text()` over the slice's tables).
3. `postgres.py` — implement the port(s) over the queries; `http.py` —
   pydantic request/response models + handlers + envelope errors;
   `module.py` — `register(app, service)`.

```python
# module.py
def register(app: FastAPI, service: UserService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
```

4. Wire at **`src/app/bin/api.py` only**: construct the adapter
   (`PgUserStore(database)`), build the service, pass it to `register`. Add
   `specs/features/<slug>.feature` for the behavioral contract.
5. Gate: `make ci` + `make accept`. If the slice shipped tables, exercise
   the new SQL against a real Postgres once by hand (`make migrate-up`, curl
   the endpoints) — the template ships no integration lane
   ([Testing & CI §3](11-testing-and-ci.md)).

## 2. Add a port + adapter pair

1. In the consuming slice's `ports.py`, declare the `Protocol` (1–2
   methods, consumer-named): `class Notifier(Protocol): async def notify(...)
   -> None: ...`.
2. Implement the adapter where the mechanism lives: an existing
   `postgres.py`, a new `platform/<x>/` module, or an in-slice fake-backed
   stub with a `# ponytail:` note naming its ceiling.
3. Wire: the root constructs the adapter; the service constructor gains one
   field. For an alternate implementation (e.g. the fake in tests), nothing
   else changes — the port is the only seam.

## 3. Ship a migration

```sh
make migrate-add NAME=add_orders_status   # alembic revision -m add_orders_status
# edit upgrade() AND downgrade() (down = true reversal, not a comment)
make migrate-up                            # apply locally
```

Rules that keep this boring: paired up/down always; no startup migrations —
the compose boot path runs the one-shot `migrations` service (or your deploy
step does the equivalent); destructive changes (drop/change column) get
their own migration with a data-migration plan noted in the file header; the
template ships `0001_init` + `0002_profiles` as the reference — in your
project, edit your own 0001 before the first deploy, never after.

`migrations/env.py` reads `<APP>_DATABASE_URL` from the environment, so the
migration path never needs `Settings` and never drifts from it.

## 3b. Run the worker with the API

`python -m app.bin.worker` is a second entrypoint on the same image (the
compose `worker` service): the email outbox loop plus the janitor (expired
sessions + stale codes, `<APP>_JANITOR_POLL_SECONDS`, default 1 h). The
claim is `FOR UPDATE SKIP LOCKED`, so api + worker + extra replicas are all
safe together. A deployment with _no_ worker quietly stops draining the
outbox and stops purging tables. Compose ships the second service; keep it
when you compose your own deploy.

## 4. Expose a new endpoint (existing slice)

1. `http.py`: handler + edge validation (a pydantic v2 model with
   `StringConstraints`), the envelope on error (raise `E(code, detail,
   status)`), and the auth posture (`IdentityDep` or
   `Depends(require_role(Role.ADMIN))`) declared in the signature.
2. `module.py`: nothing to change if the route lives on the slice's
   existing router — `include_router` already mounts it under `/api/v1`.
3. Regenerate the contract: `make generate` rewrites `specs/openapi.yaml`;
   `make gen-check` fails when it is stale.
4. Wire a new platform handle only if genuinely needed — otherwise the
   existing middleware chain (request-id, security headers, CORS) applies.
5. Acceptance scenario + unit test; `make ci`.

## 5. Add an email template

1. `app/platform/email/templates.py`: key the entry `verify_email`,
   `reset_password`, `password_changed`… — one key per entry, localized by
   locale (`en` and `es` ship today):

```python
"verify_email": {
    "en": ("Verify your email",
           "Your verification code is {code}. It expires in {hours} hours."),
    # one entry per shipped locale; unknown locales fall back to "en"
},
```

2. `{var}` placeholders interpolate with `str.format` from the message's
   `variables`; unknown locales fall back to `en` and unknown keys raise
   `ValueError` — which fails the delivery and dead-letters the job through
   the queue, visibly:

```python
message = EmailMessage(
    to=email,
    template_key="verify_email",
    variables={"code": code, "hours": self._ttl.total_seconds() / 3600},
)
```

3. Enqueue from the use case **through the store port**, so the business
   write and the outbox row commit together (§6 of
   [Persistence](08-persistence-and-migrations.md)): the adapter stages
   `enqueue_email(session, message, now)` on its own session and commits
   once. Never call a `Sender` directly from a use case (delivery is the
   worker's job; retries and dead-lettering apply uniformly).
4. Template test (render + variables) is a unit test in the platform
   package; the outbox integration lane covers claim/retry semantics once,
   not per template.

## 6. (Bonus) Replace the widgets example

Delete `src/app/features/widgets` (including its `queries.py`) + the
`widgets` table from your 0001, **or** rename it in place as your first real
slice (its publisher port is the reference wiring for hub events). Either
way, run `make ci` + `make accept` after the change — the pytest-bdd spec
`specs/features/widgets.feature` goes with the slice.

## See also

- [Project Skeleton](13-project-skeleton.md)
- [Testing & CI](11-testing-and-ci.md)
- [Design Patterns](05-design-patterns.md)
