# MEMORY.md — `<app>` premise

Durable, versioned premise for every session. Keep it under ~100 lines: facts
an agent cannot derive quickly from the code, not a copy of the code.

## Product

`<app>` is a native iOS app: Swift 6 with strict concurrency, SwiftUI, iOS 18,
one SwiftPM package per architectural layer, typed routes, and unidirectional
data flow. The template ships one example slice (`notes`) with an in-memory
adapter; replace it with the real domain. The app declares **no entitlement**,
**no `Info.plist` usage key** and talks to no network yet.

## The package graph

Six local SwiftPM packages plus one app target. The edge is a manifest fact,
not a convention.

| Package | Kind | Holds |
| --- | --- | --- |
| `Kernel` | Pure Swift | `Outcome<Value>`, `AppError`, `ErrorCode`, `NoteID` |
| `UICore` | Pure Swift | `UiStatus` — the closed status set |
| `NotesContracts` | Pure Swift | `NotesRoute`, the typed routes |
| `NotesDomain` | Pure Swift | `Note`, `NoteRepository`, `LoadNotes` |
| `NotesData` | Adapter | `InMemoryNoteRepository` |
| `NotesUI` | SwiftUI | `NotesViewState`/`NotesEvent`/`NotesEffect`, `NotesModel`, the screens |
| `<APP>` | Application | `@main`, `AppEnvironment`, `Router`, `RootView` |

Forbidden: `UI → Data`, `Domain → Data`, `Domain → UI`, `Kernel → anything`,
`Contracts → anything`, any pure package → SwiftUI or UIKit. The compiler
catches the last one; `scripts/check-architecture.sh` catches the rest.

## Routes

`NotesRoute` is a `Hashable, Sendable` enum with `list` and
`detail(id: String)`. `Router.path` is `[NotesRoute]`; `RootView` passes it to
`NavigationStack(path:)` and maps every case in `navigationDestination`. A
screen never receives a route as a `String`.

## UDF

`NotesViewState` is an immutable, `Equatable` value carrying
`status: UiStatus` plus `notes`. `NotesEvent` is `refresh | open(NoteID)`.
`NotesEffect` is one-shot and delivered through an `AsyncStream`; the app
target is the only consumer and navigates. `NotesModel` is
`@MainActor @Observable` and calls `LoadNotes`, never the repository.

## Status set

`UiStatus` has exactly eight states: `loading`, `ready`, `empty`, `offline`,
`forbidden`, `notFound`, `error(correlationID:)`, `policyBlocked`. Every
screen renders all eight; a `switch` that misses one does not compile.

## Storage and DI

The template persists nothing: `InMemoryNoteRepository` returns two fixed
notes. Wiring is explicit: `AppEnvironment.live` constructs the adapter, the
use case and the model. No DI framework, no service locator.

## Test tiers

| Tier | Where | Runs with |
| --- | --- | --- |
| kernel | `Packages/Core/Kernel/Tests` | `swift test` (macOS) |
| domain | `Packages/Feature/Notes/Domain/Tests` | `swift test` (macOS) |
| model | `Packages/Feature/Notes/UI/Tests` | `swift test` (macOS) |
| `.claude` contract | `evals/contracts.test.mjs` | `node --test` (any OS) |
| UI / integration | not shipped | add an XCUITest target when a device or simulator is available |

Swift tests use Swift Testing (`import Testing`, `@Suite`, `@Test`,
`#expect`). A model test runs on the main actor and uses a stub repository.
No test touches a real network, a real clock or a device.

## Stack

Swift **6.0** language mode · strict concurrency **complete** · iOS **18.0**
deployment target · macOS **15** (package platform for host-side `swift
test`) · SwiftUI + Observation (`@Observable`) · Swift Testing · XcodeGen
(project file generated from `project.yml`, never committed) · Node **24**
(for the contract evals only).

## Gates

`make ci` (lint + `node --test evals/contracts.test.mjs` +
`scripts/native-gates.sh`) · `make lint` (check-architecture; the `.claude`
linters and the instruction budget join it once the tree is renamed) · `make
verify` (stages a renamed copy and runs every gate on it). The native tier prints a visible `SKIP` and exits 0 on a
host without macOS + Xcode.

## Where the contracts live

`PLAYBOOK_MOBILE_IOS_SWIFT.md` (architecture, package graph, UDF, navigation,
platform integration) · `PLAYBOOK_TESTS.md` · `PLAYBOOK_SECURITY.md` ·
`PLAYBOOK_FRONTEND.md` · `PLAYBOOK_URLS.md` · `PLAYBOOK_AUDIT.md` ·
`PLAYBOOK_CLAUDE_CODE.md` · `AGENTS.md` (agent workflow) ·
`docs/dependency-register.md` (every dependency and its removal path).
