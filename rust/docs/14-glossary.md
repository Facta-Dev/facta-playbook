# Glossary

Alphabetical reference for every architectural term used in this handbook.
Generic-first.

**Adapter (driven)** — Concrete implementation of a driven port (e.g.
`PgWidgetRepository`). The only place SQL/HTTP-vendor code lives. Wired only at
the composition root.

**Adapter (driving)** — Something that calls *into* the application: an axum
handler, the CLI. Talks to a driving port (a use-case service).

**Arc<dyn Trait>** — The Rust realization of dependency injection. The field
type is the contract; the value is the concrete adapter chosen at the root.
`Send + Sync + 'static` makes it shareable across async tasks.

**Composition root** — The single crate (`<app>-api`) that knows every concrete
type. Constructs adapters, injects them into ports, builds `AppState`, mounts
routes. Nothing depends on it.

**Domain** — The innermost ring: pure types, IDs, enums, errors, value objects.
Zero I/O. Compiles in milliseconds.

**Driven port** — A trait the application calls *out to* (a repository). The
adapter implements it.

**Driving port** — The application's public API (a service struct). A driving
adapter calls it.

**Hexagonal architecture** — Ports & adapters. Dependencies point inward;
mechanisms are outside the hexagon; the domain knows nothing about them.

**Newtype / Typed-ID** — A wrapper around a primitive (`struct WidgetId(Uuid)`)
so the compiler rejects mismatched ids.

**Null Object** — A no-op implementation of a port (`NoopMailer`) used as a test
double for collaborators whose effect you don't care about.

**Offline query cache (`.sqlx/`)** — Committed metadata that lets sqlx
compile-time-check SQL without a live database. Refresh with `just sqlx-prepare`.

**Port** — A trait declaring a contract. Driven ports are repository traits;
driving ports are service entry points.

**Repository** — A driven port for persistence: `find`, `list`, `upsert`,
`delete`. Hides storage behind a trait.

**Slice (vertical)** — A self-contained use-case crate owning one feature
end-to-end over the shared domain + ports. Leaf slices don't import each other.

**Orchestrator** — The one slice (`<app>-loom`) allowed to depend on other
slices; fans a request out and stitches results together.

**Migration (paired)** — A forward `.up.sql` plus its exact inverse `.down.sql`,
so a deploy rolls back deterministically.

**Result<T>** — Workspace-wide `std::result::Result<T, core::Error>`. Every port
method returns it.

**Test double** — In-memory implementation of a port used in tests (`Mock*`).
Lives in `ports/mock/`, feature-gated.

**Vertical slicing** — Cutting the horizontal layers per-feature: each use-case
is its own crate. Composes with hexagonal layering.

## See also

- [Overview & principles](00-overview-and-principles.md)
- [Crate topology](03-crate-topology.md)