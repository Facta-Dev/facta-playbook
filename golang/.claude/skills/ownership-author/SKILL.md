---
name: "<app> Ownership Author"
description: "Review aliasing, locking, and context-propagation patterns in <app> packages; produce a line-anchored report of shared-state races, mutex placement, and Deps interface wiring across the <scope> slice before the diff lands."
division: backend
---

# <app> Ownership Author

A reusable review workflow for aliasing and concurrency correctness in the `<app>` Go module. An agent (e.g. the ownership reviewer role) invokes this skill on any diff that touches `internal/features/<slice>/**/*.go` or `internal/platform/**/*.go`; the skill is the workflow, the agent is the role. The output is a line-anchored report, not a rewrite.

## When to Activate

- A diff modifies a `Deps` struct, a use case, a handler in `http.go`, or any interface wiring in `ports.go`.
- Shared state is introduced or mutated (a hub map, an outbox worker, a cache) across goroutines.
- A `context.Context` is dropped, wrapped, or replaced in the request path, or a goroutine outlives its request.
- A reviewer asks for an ownership pass before approving a PR that crosses the net/http per-request-goroutine boundary.

Do not activate for pure formatting, doc-only, or migration-only diffs.

## Core Principles

- Compiles is not correct. A slice alias that passes `go build` and races on the second concurrent request is a bug this skill owns.
- The composition root stays thin. `cmd/api/main.go` wires consumer-defined ports into `Deps`; logic lives in the use cases.
- Never import `unsafe` (or reach for clever memory tricks) to fix sharing. The fix is a copy, a channel, or a lock.
- Cite the failure mode, not the rule. "Data race" is the rule; "races on the second concurrent request because net/http spawns a goroutine per request" is the review.
- `sync.Mutex` for short critical sections, `sync.RWMutex` when reads dominate, channels when ownership should move — never a lock held across a long I/O wait.

## Rules

- Anti-hallucination: do not fabricate file paths, line numbers, or compiler diagnostics. Every finding must cite a path and line that exists in the diff or the working tree.
- Every stat needs a source. If you claim "races on the second request", cite the net/http per-request-goroutine model; if you cite a vet check, name it (`copylocks`, `lostcancel`). No unsourced numbers.
- Do not emit the final review as a fenced code block when raw line-anchored output is expected — the report is plain text with one finding per line, severity-prefixed.
- Verify claims against the tree before reporting: read the cited file and line. A finding that cites a line the diff does not contain is a hallucination.
- `Deps` fields stay use cases and consumer-defined ports; a field that inlines a service or another slice's type is a BLOCK.
- No `unsafe` proposal; internal source importing `unsafe` is a BLOCK. Propose the safe alternative (`slices.Clone`, a channel, an explicit conversion) instead.

## Research References

Provenance table — fill the link column from the primary source before relying on a claim. No claim may appear in a finding without a row here.

| claim | source | year | link |
| --- | --- | --- | --- |
| net/http serves each request on its own goroutine | Go documentation, net/http Server | 2024 | <TODO: primary doc link> |
| go test -race detects unsynchronized shared access | Go blog, race detector | 2013 | <TODO: primary doc link> |
| Context must be the first parameter and derive from the request | Go blog, context and structs | 2014 | <TODO: primary doc link> |
| Slices/maps share backing storage across assignment | Go blog, go slices | 2013 | <TODO: primary doc link> |

## Output Format

A line-anchored report, one finding per line, severity-prefixed, followed by a summary line. Plain text — not a fenced block:

```
BLOCK  internal/platform/realtime/hub.go:42  subscribers map mutated from WS goroutines with no lock — data race on the second concurrent connection; guard with sync.RWMutex or an owner goroutine.
WARN   internal/features/widgets/postgres.go:18  List() returns a shared sqlc-built slice; callers can mutate the backing array — return slices.Clone(rows).
PASS   internal/features/widgets/usecase.go  ctx flows from r.Context() through the use case into the pgx call.
```

Summary line: `ownership-author: N block(s), M warning(s), K pass — verdict: BLOCK | APPROVE`.