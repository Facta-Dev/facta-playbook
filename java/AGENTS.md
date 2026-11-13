# `<app>` — Java / Jakarta EE backend skeleton

Runnable template for an enterprise Java backend: **Jakarta EE 11** (the stack
historically called J2EE), Java 21, Maven, CDI, JPA/Hibernate, Flyway, JAX-RS
and PostgreSQL. The architecture is **hexagonal + vertical slices**; the
`widgets` slice is the reference. The full contract is in
`PLAYBOOK_BACKEND.md` (the J2EE playbook), with `PLAYBOOK_SECURITY.md`,
`PLAYBOOK_TESTS.md`, `PLAYBOOK_URLS.md`, `PLAYBOOK_AUDIT.md` and
`PLAYBOOK_CLAUDE_CODE.md`.

## The one rule

The playbooks are restrictions, not suggestions. A change that contradicts one
is wrong until the playbook changes in the same PR.

## Layout

```text
src/main/java/com/example/app/
  shared/                 Result, AppError, TenantId, Clock
  health/                 JAX-RS health resource
  widgets/
    domain/               Widget, WidgetId, WidgetStatus — plain Java
    application/          ArchiveWidget use case + ports (repository, outbox)
    adapters/             JPA entities/repositories, outbox adapter
    presentation/         JAX-RS resource, DTO, error envelope
src/main/resources/
  META-INF/               persistence.xml, beans.xml
  db/migration/           Flyway migrations
specs/openapi.yaml        the HTTP contract
evals/                    the .claude contract suite (Python stdlib)
scripts/                  rename, verify, linters, hooks
.claude/                  agents, skills, commands, schemas, settings
docs/                     the numbered guide set
```

## Commands

- `make ci` — Checkstyle + unit/ArchUnit tests (Maven).
- `make evals` — validate the `.claude` layer and the frozen fixtures.
- `make verify` — stage a renamed copy and run every gate on it.
- `make test` / `make build` / `make run` — Maven shortcuts.
- `make fmt-check` — Checkstyle only.

## Layers (enforced by Maven + ArchUnit)

| Layer | May depend on | Must not depend on |
|---|---|---|
| domain | Java SE, `shared` | `jakarta.*`, JPA, JAX-RS |
| application | domain, ports, `jakarta.transaction`, `jakarta.inject` | JPA, JAX-RS, JMS |
| adapters | application, domain, JPA/JMS clients | presentation |
| presentation | application, JAX-RS, validation | adapters, JPA |

## Rules of the kit

1. The **use case** owns the JTA transaction; the resource is thin.
2. JPA entities never cross the API boundary; DTOs are records.
3. Every query carries the tenant predicate; `tenant_id` + RLS are the
   database second line.
4. The domain row and the outbox row commit together; every async effect is
   idempotent.
5. CDI construction is explicit (constructor injection); no service locator.
6. No secret in config, logs, images or the repository.
7. No `// CHECKSTYLE:OFF` or `@SuppressFBWarnings` without a reason.

## Workflow

Spec → test → code. Use `/spec`, `/plan`, `/review` and `/ship` (or the matching
skills). Never code before a red test. Run `make ci` before declaring done; run
`make verify` before a release.
