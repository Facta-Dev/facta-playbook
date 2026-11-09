---
name: "<app> Add Handler"
description: "Scaffold a new net/http handler route in <app>: handler decode shape, envelope mapping, specs/openapi.yaml entry, route registration in internal/features/<slice>/module.go, and Deps wiring, as one change across the slice."
division: backend
---

# <app> Add Handler

A reusable scaffolding workflow for adding an HTTP capability to the `<app>` API. An agent (e.g. the API designer role) invokes this skill to produce the handler, the OpenAPI spec entry, the route registration, and the Deps wiring together — so a route never lands without its spec and its wiring. The output is a design sketch plus a registration checklist.

## When to Activate

- A new endpoint is requested on the `<app>` API (a new verb on an existing path, or a new path).
- An existing handler needs a new decode shape or a new error code.
- The OpenAPI spec must register a route that already exists in code (drift fix).
- A reviewer asks for the handler + spec + wiring bundle before approving an API change.

Do not activate for non-HTTP changes (domain logic, persistence, migrations) — those belong to other skills.

## Core Principles

- The handler is an adapter, not a service. Decode, validate at the edge, call the slice's use case (`internal/features/<slice>/usecase.go`), map to the envelope, return. No SQL, no domain logic in the handler.
- The spec is the contract; the models are generated. `specs/openapi.yaml` is hand-edited as the source of truth; `internal/gen/api` is the artifact `make generate` refreshes. A route that lands without its spec entry is a defect.
- The composition root stays thin. A new `Deps` field is a use case / small consumer-defined interface wired once in `cmd/api/main.go`; logic lives in the use cases.
- Registration is centralized per slice. `internal/features/<slice>/module.go` `Register(mux, d)` is the one place a slice's routes are mounted.
- Errors are typed, not stringy. Every domain error the use case can return maps to a stable `apperr` code with a deliberate status code.

## Rules

- Anti-hallucination: do not invent package paths, use-case methods, or error codes that are not in the tree. Every wiring step cites a file that exists.
- Every stat needs a source. If you cite the ServeMux method+wildcard precedence rule, cite the net/http docs; if you cite a status code semantics, cite RFC 9110. No unsourced numbers.
- Do not emit the final design as a fenced code block when raw output is expected — the sketch is plain text with labelled sections (handler / spec / wiring / checklist).
- Verify the cited use-case method exists in `internal/features/<slice>/usecase.go` before wiring it; a handler that calls a method the use case does not expose is a hallucination.
- Every route must carry a matching `specs/openapi.yaml` entry; `make generate` must stay green (committed `internal/gen/api` has no drift).
- Routes are registered in the slice's `module.go`, never inline in `cmd/api/main.go` or in another slice's file.
- Auth-protected handlers sit behind `authx.RequireAuth` / `authx.RequireRole`; public handlers are explicitly marked and rare.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| net/http ServeMux method+wildcard patterns route by most-specific match | Go documentation, net/http ServeMux | 2023 | <TODO: primary doc link> |
| specs/openapi.yaml is the source of truth for internal/gen/api models | oapi-codegen documentation | 2024 | <TODO: primary doc link> |
| HTTP status code semantics (400/401/403/404/409/500) | RFC 9110, status codes | 2022 | <TODO: primary doc link> |
| authx.RequireAuth stores Identity via WithIdentity on the request context | internal/platform/authx (local) | — | local file |

## Output Format

A design sketch plus a registration checklist, anchored to the skeleton file layout. Plain text — not a fenced block:

```
DESIGN  POST /api/v1/widgets  (internal/features/widgets/http.go)

  handler:
    func (h Handler) create(w http.ResponseWriter, r *http.Request) {
        var req api.CreateWidgetRequest            // model from internal/gen/api
        if err := httpx.DecodeJSON(w, r, &req); err != nil { return }
        // validate at the edge (validator/v10), call use case, write envelope
    }

  spec:
    specs/openapi.yaml — add the request/response schemas + the
    POST /api/v1/widgets path (operationId: createWidget); run
    make generate and commit the regenerated internal/gen/api.

  wiring:
    module.go  -> mux.HandleFunc("POST /api/v1/widgets", h.create)
    deps       -> Deps.Create use case (already wired; no new field)

CHECKLIST
  [ ] handler is a thin adapter — no business logic; calls the use case
  [ ] specs/openapi.yaml entry added; make generate run; internal/gen/api committed
  [ ] route registered in module.go Register() (not inline in main.go)
  [ ] envelope {"detail","code"} covers the failure modes; no leaky 500s
  [ ] Deps unchanged OR new field is a use case / port wired in main.go
```

Summary line: `add-handler: N handler(s) designed, M registration step(s) — verdict: APPROVE | REVISE`.