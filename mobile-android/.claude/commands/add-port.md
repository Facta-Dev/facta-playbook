---
name: add-port
description: "Delegate to the add-port skill — add a port to the <app> Android template: the consumer-defined interface in :feature:notes:domain, the adapter in :feature:notes:data, the manual wiring in :app and the fake for the domain tests."
division: core
---

# /add-port

## Delegate

Delegate to the **add-port** skill. Activate when a use case in
`:feature:notes:domain` needs a capability that lives outside the domain.

Steps:

1. Check the port earns its place: name the consumer, the capability and the
   smallest signature that consumer actually calls. A trivial wrapper is not a
   port.
2. Declare the interface in `:feature:notes:domain`, next to the use case, in
   domain types only — no `Context`, no `android.*`, no framework error type
   — returning `Outcome<T>` and never swallowing `CancellationException`.
3. Implement it in `:feature:notes:data`, the only module that knows about the
   SDK, the cache or the platform.
4. Wire it by hand in `:app`: construct the implementation and pass it to the
   ViewModel through the explicit factory. A port with no wiring is dead code.
5. Write the fake in the test source set and cover the domain use case with
   it, so the domain suite never touches Android.
6. Run `./gradlew :app:assembleDebug`, `./gradlew test`, `./gradlew :evals:test`
   and `bash scripts/check-architecture.sh`, and report the real output.
