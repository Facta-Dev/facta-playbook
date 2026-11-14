# `<app>` — Java / Jakarta EE backend template

A runnable enterprise Java backend on **Jakarta EE 11** (J2EE), Java 21, Maven,
CDI, JPA/Hibernate, Flyway, JAX-RS and PostgreSQL. It implements the facta
hexagonal + vertical-slice contract with a reference `widgets` slice.

> Companion repo: [beautiful-j2ee](https://github.com/Facta-Dev/beautiful-j2ee) —
> the Agent Skills and linter config for this stack.

## Quick start

```bash
cp .env.example .env
mvn verify                 # Checkstyle + unit + ArchUnit tests
mvn package                # target/app.war
docker compose up --build   # WildFly + PostgreSQL
```

Without a local Maven/application server, `make evals` validates the `.claude`
layer and `make verify` stages a renamed copy and runs the structural gates
(Maven stages SKIP visibly).

## What is inside

- `widgets` slice: domain, `ArchiveWidget` use case, JPA adapter, outbox and a
  JAX-RS resource (`POST /api/v1/app/widgets/{id}/archive`).
- `Result`/`AppError` and the platform error envelope.
- Flyway migration with `tenant_id`, an index and PostgreSQL RLS.
- JUnit 5 use-case tests and an ArchUnit layer test.
- `.claude/` agents, skills and commands for spec, plan, review and ship.
- The six playbooks, the numbered docs and the ADRs.

## Layout

See `AGENTS.md` for the layer rules and `docs/13-project-skeleton.md` for the
tree. The HTTP contract is `specs/openapi.yaml`.

## Gates

| Command | Proves |
|---|---|
| `make ci` | Checkstyle + unit/ArchUnit tests |
| `make evals` | `.claude` agents/skills/commands against the schemas, frozen fixtures |
| `make verify` | staged renamed copy + every gate |

`scripts/verify-template.sh` never mutates the working tree. A missing Maven
prints a visible `SKIP`; CI installs it and runs the same gates.

## Rename

```bash
bash scripts/rename.sh myapp acme
mvn verify
```

Substitutes `<app>`, `<APP>` and `<scope>` outside `scripts/`, `.claude/` and
`evals/fixtures/`. The Java package stays `com.example.app`; rename it to your
own package once, before the first release.

## License

MIT (inherited from the ai-coding-playbook repository).
