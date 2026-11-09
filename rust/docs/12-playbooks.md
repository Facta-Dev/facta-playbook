# Playbooks (How-To)

Copy-pasteable, step-by-step recipes for the common evolution tasks.

## 1. Bootstrap a hexagonal workspace

1. Copy `skeleton/` from this template.
2. Global-replace `<app>` → your project name (dirs, `Cargo.toml`, `use` paths).
3. `cargo generate` or `git init` in the renamed dir; `cargo check --workspace`.
4. Set `DATABASE_URL`; `just sqlx-prepare`; commit `.sqlx/`.
5. `just ci` — green.

## 2. Add a vertical slice (`<scope>`)

1. `crates/<app>-<scope>/` with a `Cargo.toml` inheriting `[lints] workspace = true`.
2. If it needs new storage: add a trait to `ports/src/domain/<scope>_repository.rs`,
   a `Pg*` adapter to `ports/src/adapters/<scope>_pg.rs`, a mock to
   `ports/src/mock/<scope>_mock.rs`; re-export from `ports/src/lib.rs`.
3. Write the `<Scope>Service` holding `Arc<dyn ...>`; unit-test with the mock.
4. Add the crate to root `[workspace] members` and `[workspace.dependencies]`.
5. Wire it into `AppState::from_env`; mount routes in `router::build`.
6. Add a paired migration (`just migrate-add create_<scope>`); refresh `.sqlx/`.
7. `just ci`.

## 3. Add a driven adapter (infra)

1. `crates/<app>-<thing>/` — a trait (`<Thing>`), a real impl, and a `Noop<Thing>`
   double.
2. Depend on `core` only (for `Error`/`Result`).
3. Expose a `with_endpoint`/`with_http` test hook so adapter tests aim at a mock
   server.
4. Wire the real impl in `AppState::from_env`; the `Noop` in tests.
5. Add to `[workspace] members` + `[workspace.dependencies]`.

## 4. Add an auth method

1. Add the credential model + a `*Repository` port in `<app>-ports`.
2. Extend the auth slice (`<app>-warden` or equivalent) with the new flow.
3. Map provider errors into `core::Error::Upstream` inside the adapter.
4. Add a middleware step in the authenticate → rate-limit → authorize pipeline.
5. Unit-test the flow with the mock repo + `NoopMailer`.

## 5. Ship a reversible migration

1. `just migrate-add <name>` → creates `NNNN_<name>.up.sql` + `.down.sql`.
2. Write the forward SQL; write the exact inverse in `.down.sql`.
3. Apply locally; add/adjust adapter SQL; `just sqlx-prepare`; commit `.sqlx/`.
4. `just ci`. The down migration is tested on rollback drills, not every deploy.

## 6. Expose a new endpoint

1. Add the use-case method to the slice's `Service`.
2. Add a handler in `<app>-api` that extracts the request, calls the service,
   maps `core::Error` → HTTP.
3. Mount the route in `router::build`.
4. Smoke test: boot the router with `AppState::with_repos` (mocks) and hit the
   route.
5. Mirror the wire contract in `<app>-sdk`.

## See also

- [Project skeleton](13-project-skeleton.md)
- [Vertical slicing](02-vertical-slicing.md)
- [Testing & CI](11-testing-and-ci.md)