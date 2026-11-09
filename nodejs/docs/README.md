# Backend Node Architecture Handbook

A **generic, reusable** reference for building a Node.js backend with
**hexagonal architecture (ports & adapters) + vertical slicing**, realized as a
single ESM TypeScript package (Node 24, Fastify 5, pg, Kysely,
node-pg-migrate). Every chapter teaches the pattern abstractly first, then
shows a worked example (`### Worked example`) so you can lift the structure
into any project. Replace `<app>` with your project name and `<scope>` with
your org/module prefix throughout.

> 15 chapters · adapted to TS/Node idioms (structural typing, subpath imports,
> `AbortSignal` cancellation, fastify hooks, zod at the edges).

## The one rule

**Dependencies point inward.** The domain knows nothing about I/O; the
composition root (`src/bin/api.ts`) knows about everything and wires it
together. Everything else makes that rule cheap to follow and hard to break.
See [Dependency Rules](04-dependency-rules.md).

```mermaid
graph LR
  ROOT[Composition root] --> UC[Use-cases] --> PORTS[Ports] --> DOMAIN[Domain]
  ADAPTERS[Adapters] -. implement .-> PORTS
  ROOT -.wires.-> ADAPTERS
  classDef d fill:#1b5e20,color:#fff
  class DOMAIN,PORTS d
```

## Start here

New? Read [00 — Overview & Principles](00-overview-and-principles.md), then
[01 — Hexagonal Architecture](01-hexagonal-architecture.md) and
[03 — Package Topology](03-package-topology.md). Starting a fresh project?
Jump to [13 — Project Skeleton](13-project-skeleton.md) and
[12 — Playbooks](12-playbooks.md).

## Contents

| #   | Chapter                                                            | What it covers                                                                                    |
| --- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| 00  | [Overview & Principles](00-overview-and-principles.md)             | The one idea + 8 principles + chapter map                                                         |
| 01  | [Hexagonal Architecture](01-hexagonal-architecture.md)             | Ports & adapters, consumer-declared interfaces, dependency inversion                              |
| 02  | [Vertical Slicing](02-vertical-slicing.md)                         | One feature per package over a shared kernel; the seven shipped slices                            |
| 03  | [Package Topology](03-package-topology.md)                         | Module taxonomy, the import graph, role→module map                                                |
| 04  | [Dependency Rules](04-dependency-rules.md)                         | Allowed/forbidden edges, enforced by `no-restricted-imports` in `eslint.config.mjs`               |
| 05  | [Design Patterns](05-design-patterns.md)                           | Options objects, fastify hook chains, adapter classes — TS/Node-idiomatic                         |
| 06  | [Auth Architecture](06-auth-architecture.md)                       | bcrypt+pepper, JWT + rotating refresh, codes, RBAC, audit                                         |
| 07  | [Request Lifecycle](07-request-lifecycle.md)                       | Hooks → identity → use case → port → adapter; WS/SSE aside                                        |
| 08  | [Persistence & Migrations](08-persistence-and-migrations.md)       | pg Pool, Kysely per-slice queries, paired reversible node-pg-migrate migrations, never at startup |
| 09  | [Realtime & Streaming](09-realtime-and-streaming.md)               | Hub contract, WebSocket frames + close 4001, SSE hijack flushing, Redis evolution                 |
| 10  | [Errors, Config & Observability](10-error-config-observability.md) | `{detail, code}` envelope, zod env Load, pino-json logs, health gates, `pnpm audit`               |
| 11  | [Testing & CI](11-testing-and-ci.md)                               | Test pyramid, fakes over mocks, cucumber acceptance, `make ci`                                    |
| 12  | [Playbooks](12-playbooks.md)                                       | Add a slice, add a port+adapter, ship a migration, expose an endpoint, add an email template      |
| 13  | [Project Skeleton](13-project-skeleton.md)                         | Annotated tree of the shipped `skeleton/`                                                         |
| 14  | [Glossary](14-glossary.md)                                         | Every architectural term defined                                                                  |

## How to apply this to a new project

1. Copy the package layout from [Project Skeleton](13-project-skeleton.md).
2. Put your types and rules in the **domain** files (`domain.ts`, stdlib +
   `#shared/*` only).
3. Declare what you need from the world as **consumer-defined interfaces**
   in `ports.ts`; implement them as **adapters** wired only at the
   **composition root**.
4. Add each feature as a **vertical slice** under `src/features/`.
5. Keep the [dependency rule](04-dependency-rules.md) green;
   `no-restricted-imports` makes violations red, not reviewable.
