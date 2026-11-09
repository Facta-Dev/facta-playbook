# Vertical Slicing

Layering is horizontal (domain → ports → use case → adapters). **Vertical
slicing** cuts the other way: every feature owns its full stack in one
directory. This template ships seven slices; you replace the example and add
your own.

## 1. The unit: `src/features/<slice>/`

A slice is a _capability_, not a table. It contains everything that churns
together, top-down:

```
src/features/widgets/
├── domain.ts       pure types + rules + errors (#shared/* only)
├── ports.ts        the interfaces this feature needs (consumer-defined)
├── usecase.ts      orchestration: rules first, I/O last
├── http.ts         Fastify adapters, edge validation, envelope
├── postgres.ts     Pool-backed adapter over this slice's queries
├── queries.ts      typed per-slice queries (the sqlc analog — Kysely-style)
├── usecase.test.ts unit tests beside the code they cover
└── module.ts       register*(app, Deps) — the wiring contract
```

Slices are "tall but narrow": from the HTTP route down to the domain rows,
with zero sideways imports. Tests, queries and specs live beside
(`usecase.test.ts`, the slice's `queries.ts`, `specs/features/*.feature`).

## 2. The shipped seven

| Slice          | Owns                                                     | Tables                            |
| -------------- | -------------------------------------------------------- | --------------------------------- |
| `authsignup`   | signup (idempotent), verify email, resend code           | `users`, `email_verifications`    |
| `authsession`  | login, refresh (rotation), logout, me                    | `sessions`                        |
| `authpassword` | forgot/validate/reset code + rate limit, change password | `password_resets`                 |
| `adminusers`   | admin list/get/update users, audit logging               | `users` (admin ops), `audit_logs` |
| `authoauth`    | Google One Tap sign-in + GitHub OAuth/link flows         | provider-id columns on `users`    |
| `widgets`      | worked example: CRUD + SSE lifecycle events              | `widgets`                         |
| `realtime`     | WebSocket endpoint `/api/v1/ws?token=`                   | — (hub lives in platform)         |

Boundaries: `authsignup` and `authpassword` never call `authsession`; login
and reset can both need to revoke sessions — the _capability_ is duplicated
through a port, or the shared method is promoted into `shared/`/`platform/`
(kernel-only). If two slices keep reaching for the same port, that is the
signature of a missing shared port, not an import to add.

### How coarse is coarse enough?

One slice ≈ one bounded feature (a group of use cases over one aggregation).
The signup trio (signup/verify/resend) churns together — one slice. Splitting
`verify` into its own directory would triple the boilerplate for nothing;
merging all three auth slices would bury `signup` under `password` code. When
a slice's `domain.ts` passes ~200 lines, that is usually two features sharing
a directory — split by behavior, not by file count.

## 3. Slices never import each other

```mermaid
graph LR
    subgraph slices
        AUTH[authsignup] ; SES[authsession] ; W[widgets] ; R[realtime]
    end
    subgraph shared-horizontal
        SH[shared/ kernel] ; PL[platform/]
    end
    AUTH --> SH & PL
    SES --> SH & PL
    W --> SH & PL & R["(SSE via hub port, not import)"]:::plat
    classDef plat fill:#37474f,color:#fff
```

The widgets slice publishes `created/updated/deleted` into the hub **through
an event-bus port** — it does not import the `realtime` slice's route. The
realtime slice and the platform hub are the same mechanism at different
altitudes: the _port_ (narrow event bus) lives with the consumer, the _hub
implementation_ lives in `src/platform/realtime`.

## 4. Adding a slice

The recipe is [Playbook §1](12-playbooks.md); the checklist: mkdir, the eight
files, queries + `make migrate-add NAME=<table>` (pair ships its `down`) +
`make generate` if the OpenAPI contract grew, cucumber spec if behavioral,
wire in `src/bin/api.ts`, `make ci`. Nothing else in the codebase changes — no
registry edits, no god files. If adding your slice edits three files outside
`src/features/` and `src/bin/`, the template regressed.

## 5. Anti-patterns

- **Layer-packages across the whole app** (`src/repos/`, `src/services/`,
  `src/handlers/`): the slice is replaced by a _horizontal_ directory set, so
  a feature touches five trees. The eight files per slice exist precisely to
  kill those god-directories.
- **Cross-slice imports "temporarily"**: eslint denies `#features/*` imports
  from inside features. The fix is a port or a kernel promotion, never an
  exception comment.
- **A slice that owns another slice's table**: ownership follows capability.
  `adminusers` may update `users`, but it does not own the signup flow — it
  consumes an admin-shaped port, and its queries are its own, in its own
  `queries.ts`.
- **Shared mutable singletons in slices**: per-slice state is a dependency of
  `register*`; anything global should live in `platform/` and be injected.

## See also

- [Hexagonal Architecture](01-hexagonal-architecture.md)
- [Package Topology](03-package-topology.md)
- [Dependency Rules](04-dependency-rules.md)
