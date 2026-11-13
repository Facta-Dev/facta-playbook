---
name: add-port
description: "Delegate to the add-port skill — add a domain port to the <app> iOS template: the protocol in Domain, the adapter in Data, the stub in the test, the composition-root wiring and the use-case test."
division: core
---

# /add-port

## Delegate

Delegate to the **add-port** skill. Activate when a use case needs a
capability the domain must not implement: load or save data, read the clock,
send analytics, access the Keychain, call the network.

Steps:

1. Write the protocol next to the use case that needs it, naming only the
   operations that use case calls; the protocol is `Sendable` and its methods
   are `async throws`.
2. Implement it in `Packages/Feature/<slice>/Data`; the adapter may reach
   URLSession, SwiftData or the Keychain, and imports no UI framework.
3. Add a struct stub implementing the port with a fixed result and one that
   throws `AppError`, next to the test — no mocking framework.
4. Wire it in `AppEnvironment`: construct the adapter and pass it into the use
   case.
5. Make the use case catch `AppError` and map unknown errors to `.unknown`.
6. Add a `docs/dependency-register.md` entry when the adapter pulls a remote
   or platform dependency.
7. Run `make ci` and `bash scripts/check-architecture.sh`, and report the
   real output.
