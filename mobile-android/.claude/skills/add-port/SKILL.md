---
name: "add-port"
description: "Adds a port to the <app> Android template: the consumer-defined interface in :feature:notes:domain, its implementation in :feature:notes:data, the hand-made wiring in :app, and the fake that lets the domain test run without Android."
division: core
---

# `<app>` add-port skill

> Reusable workflow for adding one port — a consumer-defined interface plus its
> adapter — to `<app>`. An agent invokes this skill; the requester supplies the
> capability the app needs, the skill supplies the interface, the
> implementation, the wiring and the fake.

## When to Activate

Activate this skill when a use case in `:feature:notes:domain` needs a
capability that lives outside the domain: the network, the local cache, the
device, the clock, secure storage, or a push channel.

Do NOT activate this skill when:

- The capability is a pure function of the domain — it belongs in the domain
  and needs no port.
- The request is to change an existing port's signature — that is a change in
  `:feature:notes:domain` with a red test first.
- The request is to add a slice — that is `add-slice`, which creates the
  ports the new slice needs.

## Core Principles

- **The consumer owns the interface.** The port is declared in
  `:feature:notes:domain`, next to the use case that needs it, with the
  smallest signature that use case actually calls.
- **The domain stays pure.** A port never mentions `android.*`, a Retrofit
  service, a Room DAO or a `Context`; it speaks in domain types and returns
  `Outcome<T>`.
- **The adapter lives in `:feature:notes:data`.** It is the only place that
  knows about the SDK, Room, DataStore or the platform.
- **`:app` wires by hand.** This template has no DI framework: the graph in
  `:app` constructs the implementation and hands it to the ViewModel through
  an explicit `ViewModelProvider.Factory`.
- **A port earns its existence.** One implementation and one consumer is
  still the right shape — but a port with a single trivial implementation and
  no test value is a smell, and the skill says so before adding it.
- **A fake proves the domain.** The domain test uses a fake implementation in
  the test source set, so the domain suite never touches Android.

## Rules

- No fabricated paths, types or endpoint names: the interface, its methods
  and its domain types are written exactly as they will exist.
- The port returns `Outcome<T>` (`Success` or `Failure(AppError)`); it does
  not throw for expected failures, and `CancellationException` is always
  rethrown.
- The port has no Android type in any signature, including `Context`,
  `Flow<Resource<T>>` or a framework-specific error type.
- The implementation lives in `:feature:notes:data`; the UI module never
  imports it.
- The wiring change in `:app` is part of the same change: a port with no
  implementation wired into the graph is dead code.
- The port ships with a fake in the test source set and at least one domain
  test that uses it.
- No new third-party dependency without a reason; the porting must not drag in
  a networking or persistence library that the template does not already use.
- The change is not done until `./gradlew :app:assembleDebug`, `./gradlew test`,
  `./gradlew :evals:test` and `bash scripts/check-architecture.sh` are green,
  and the real output is reported.

## Research References

| Claim | Source | Year | Link |
|-------|--------|------|------|
| Ports are consumer-defined and live with the use case | `PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` §4.3 | - | `../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` |
| A pure Kotlin (JVM) module carries no Android manifest and no Android dependency | Kotlin documentation — Share code between platforms | 2026 | https://kotlinlang.org/docs/multiplatform-share-on-platforms.html |
| Room is the local cache and DataStore the preferences store | Android Developers — Room | 2026 | https://developer.android.com/training/data-storage/room |
| Kotlin structured concurrency requires cancellation to propagate | Kotlin documentation | 2026 | https://kotlinlang.org/docs/cancellation-and-timeouts.html |
| Dependency inversion keeps the domain independent of the platform | Kotlin documentation | 2026 | https://kotlinlang.org/docs/coding-conventions.html |

## Output Format

```
Port: <Name> — <module: :feature:notes:domain>
Signature:
  <fun> — <params> — <Outcome<T>>
Domain types used: <type> — <module>
Implementation: <Name>Impl — <module: :feature:notes:data> — <what it talks to>
Wiring: <what :app now constructs and injects>
Fake: <name> — <test source set path>
Tests: <the domain cases that use the fake>
Gates:
  ./gradlew :app:assembleDebug -> <result>
  ./gradlew test -> <result>
  ./gradlew :evals:test -> <result>
  bash scripts/check-architecture.sh -> <result>
Open: <anything left, such as a second implementation that may be needed later>
```
