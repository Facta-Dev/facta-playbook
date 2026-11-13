---
name: "add-port"
description: "Adds a domain port to the <app> iOS template: the protocol in Domain, the adapter in Data, the stub in the test, the composition-root wiring and the use-case test."
division: core
---

# `<app>` add-port skill

> A port is what the domain needs, not what the SDK offers. This skill adds
> one with its adapter, its stub and its wiring.

## When to Activate

Activate when a use case needs a capability the domain must not implement:
load or save data, read the clock, send analytics, access the Keychain, call
the network.

Do NOT activate when:

- The capability is a pure computation; it belongs in the domain as a
  function.
- The port already exists; extend the protocol and its adapters instead.

## Core Principles

- **Consumer-defined.** The protocol lives next to the use case that needs it
  and names only the operations that use case calls.
- **The adapter implements, the domain decides.** `Data` satisfies the port;
  the domain never imports the SDK type the adapter uses.
- **The stub is a first-class test double.** A struct implementing the port
  with a fixed result lives next to the test; no mocking framework.
- **Wiring is explicit.** `AppEnvironment` constructs the adapter and passes
  it into the use case.
- **Async and throwing at the boundary.** The port expresses failure as a
  thrown `AppError`; the use case converts it to an `Outcome`.

## Rules

- The protocol is `Sendable` and its methods are `async throws`.
- The adapter lives in `Packages/Feature/<slice>/Data` and imports no UI
  framework.
- The use case catches `AppError` and maps unknown errors to `.unknown`.
- Tests: the use case with a success stub, an `AppError` stub and a generic
  throwing stub.
- A remote or platform dependency added by the adapter gets a
  `docs/dependency-register.md` entry.
- Gates: `make ci`, `bash scripts/check-architecture.sh`.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Ports and adapters keep the domain independent | `PLAYBOOK_MOBILE_IOS_SWIFT.md` §4 | - | `../PLAYBOOK_MOBILE_IOS_SWIFT.md` |
| Protocol-oriented seams are testable without a framework | Swift.org — Protocols | 2026 | https://docs.swift.org/swift-book/documentation/the-swift-programming-language/protocols/ |
| Structured concurrency keeps the boundary async | Swift.org — Concurrency | 2026 | https://docs.swift.org/swift-book/documentation/the-swift-programming-language/concurrency/ |

## Output Format

```
Port: <ProtocolName> — <methods>
Domain: <file> — the protocol and the use case that consumes it
Adapter: <file> — <the SDK or store it reaches>
Stub: <file> — <fixed result / throwing>
Wiring: AppEnvironment — <constructor call>
Tests: <file> — <success, AppError, unknown>
Gates:
  make ci -> <result>
  bash scripts/check-architecture.sh -> <result>
```
