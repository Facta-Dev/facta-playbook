---
name: "<app> Add Handler"
description: "Scaffold a new Fastify route in <app>: handler decode shape, envelope mapping, specs/openapi.yaml entry, route registration in src/features/<slice>/module.ts, and Deps wiring, as one change across the slice."
division: backend
---

# <app> Add Handler

A reusable scaffolding workflow for adding an HTTP capability to the `<app>` API. An agent (e.g. the API designer role) invokes this skill to produce the handler, the OpenAPI spec, the route registration, and the Deps wiring together — so a route never lands without its spec and its wiring. The output is a design sketch plus a registration checklist.

## When to Activate

- A new endpoint is requested on the `<app>` API (a new verb on an existing path, or a new path).
- An existing handler needs a new decode shape or a new error code.
- The OpenAPI spec must register a route that already exists in code (drift fix).
- A reviewer asks for the handler + spec + wiring bundle before approving an API change.

Do not activate for non-HTTP changes (domain logic, persistence, migrations) — those belong to other skills.

## Core Principles

- The handler is an adapter, not a service. Decode, validate at the edge, call the slice's use case (`src/features/<slice>/usecase.ts`), map to the envelope, return. No SQL, no domain logic in the handler.
- The spec is the contract; the models are generated. `specs/openapi.yaml` is hand-edited as the source of truth; `src/gen/api` is the artifact `make gen` refreshes. A route that lands without its spec entry is a defect.
- The composition root stays thin. A new `Deps` field is a use case / small consumer-defined interface wired once in `src/bin/api.ts`; logic lives in the use cases.
- Registration is centralized per slice. `src/features/<slice>/module.ts` `register(app, d)` is the one place a slice's routes are mounted.
- Errors are typed, not stringy. Every domain error the use case can return maps to a stable `apperr` code with a deliberate status code.

## Rules

- Anti-hallucination: do not invent import paths, use-case methods, or error codes that are not in the tree. Every wiring step cites a file that exists.
- Every stat needs a source. If you cite Fastify's one-handler-per-method+path rule (duplicates throw FST_ERR_DUPLICATED_ROUTE), cite the Fastify docs; if you cite a status code semantics, cite RFC 9110. No unsourced numbers.
- Do not emit the final design as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (handler / spec / wiring / checklist).
- Verify the cited use-case method exists in `src/features/<slice>/usecase.ts` before wiring it; a handler that calls a method the use case does not expose is a hallucination.
- Every route must carry a matching `specs/openapi.yaml` entry; `make gen-check` must stay green (committed `src/gen/api` has no drift).
- Routes are registered in the slice's `module.ts`, never inline in `src/bin/api.ts` or in another slice's file.
- Auth-protected handlers sit behind the `authx` require-auth preHandler / require-role guard; public handlers are explicitly marked and rare.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim                                                                                     | source                           | year | link                     |
| ----------------------------------------------------------------------------------------- | -------------------------------- | ---- | ------------------------ |
| Fastify registers one handler per method+path (duplicates throw FST_ERR_DUPLICATED_ROUTE) | Fastify documentation, Routing   | 2024 | <TODO: primary doc link> |
| specs/openapi.yaml is the source of truth for src/gen/api models                          | openapi-typescript documentation | 2024 | <TODO: primary doc link> |
| HTTP status code semantics (400/401/403/404/409/500)                                      | RFC 9110, status codes           | 2022 | <TODO: primary doc link> |
| httpx.decode validates the request body with a zod schema at the trust boundary           | src/platform/httpx (local)       | —    | local file               |

## Output Format

A design sketch plus a registration checklist, anchored to the skeleton file layout. Plain text — not a fenced block:

```
DESIGN  POST /api/v1/widgets  (src/features/widgets/http.ts)

  handler:
    app.post("/api/v1/widgets", { preHandler: requireAuth }, async (req, reply) => {
      const body = httpx.decode(CreateWidgetRequest, req.body)
      // validate at the edge (zod), call the use case, write the envelope
    })

  spec:
    specs/openapi.yaml — add the request/response schemas + the
    POST /api/v1/widgets path (operationId: createWidget); run
    make gen and commit the regenerated src/gen/api.

  wiring:
    module.ts  -> register(app, d) mounts POST /api/v1/widgets
    deps       -> Deps.create use case (already wired; no new field)

CHECKLIST
  [ ] handler is a thin adapter — no business logic; calls the use case
  [ ] specs/openapi.yaml entry added; make gen run; src/gen/api committed
  [ ] route registered in module.ts register() (not inline in the bin)
  [ ] envelope {"detail","code"} covers the failure modes; no leaky 500s
  [ ] Deps unchanged OR new field is a use case / port wired in src/bin/api.ts
```

Summary line: `add-handler: N handler(s) designed, M registration step(s) — verdict: APPROVE | REVISE`.
