# MEMORY.md — `<app>` premise

Durable, versioned premise for every session. Keep it under ~100 lines: facts
an agent cannot derive quickly from the code, not a copy of the code.

## Product

`<app>` is a native Android app: Kotlin 2.4 + Jetpack Compose, one Gradle
module per architectural layer, Navigation 3 typed routes, and unidirectional
data flow. The template ships one example slice (`notes`) with an in-memory
adapter; replace it with the real domain. The app declares **zero Android
permissions** and talks to no network yet.

## The module graph

Eight Gradle modules. The edge is a build fact, not a convention.

| Module | Kind | Holds |
| --- | --- | --- |
| `:app` | Android application | The `Application` (absent in the template), the manual graph, `MainActivity`, the `NavDisplay` host, the adapter choice |
| `:core:kernel` | Pure JVM | `Outcome<T>`, `AppError`, `ErrorCode`, `NoteId` |
| `:core:ui-core` | Android library | `UiStatus` — the closed status set |
| `:feature:notes:contracts` | Pure JVM | `NotesRoute`, the `@Serializable` `NavKey` routes |
| `:feature:notes:domain` | Pure JVM | `Note`, `NoteRepository`, `LoadNotes` |
| `:feature:notes:data` | Android library | `InMemoryNoteRepository` |
| `:feature:notes:ui` | Android library | `NotesState`/`NotesEvent`/`NotesEffect`, `NotesViewModel`, the screens |
| `:evals` | Pure JVM | The `.claude` contract tests |

Forbidden: `ui → data`, `domain → data`, `domain → ui`, `kernel → anything`,
any pure JVM module → an Android module. The compiler catches the last one;
`scripts/check-architecture.sh` catches the rest.

## Routes

`NotesRoute` is a `@Serializable sealed interface : NavKey` with
`List` and `Detail(id: String)`. `:app` registers both subclasses in a
`SerializersModule` and passes it to `rememberNavBackStack(SavedStateConfiguration,
NotesRoute.List)`. `NavDisplay` maps each key to a screen. A screen never
receives a route as a `String`.

## UDF

`NotesState` is `@Immutable` and carries `status: UiStatus` plus `notes`.
`NotesEvent` is `Refresh | Open(NoteId)`. `NotesEffect` is one-shot and
delivered through `Channel(BUFFERED)`; `:app` collects it and navigates.
The ViewModel calls `LoadNotes`, never the repository; it takes a
`CoroutineDispatcher` in the constructor.

## Status set

`UiStatus` has exactly eight states: `Loading`, `Ready`, `Empty`, `Offline`,
`Forbidden`, `NotFound`, `Error(correlationId)`, `PolicyBlocked`. Every screen
renders all eight; a `when` that misses one does not compile.

## Storage and DI

The template persists nothing: `InMemoryNoteRepository` returns two fixed
notes. Manual DI: `:app` constructs the adapter and the use case, and the
ViewModel is built with `viewModel { NotesViewModel(LoadNotes(repo)) }`. No
Hilt, no Koin, no service locator.

## Test tiers

| Tier | Where | Runs with |
| --- | --- | --- |
| kernel | `core/kernel/src/test` | `./gradlew test` |
| domain | `feature/notes/domain/src/test` | `./gradlew test` |
| viewmodel | `feature/notes/ui/src/test` | `./gradlew testDebugUnitTest` |
| `.claude` contract | `evals/src/test` | `./gradlew :evals:test` |
| instrumented | not shipped | add `androidTest` with a device |

A ViewModel test uses `kotlinx-coroutines-test` and covers the status mapping
plus the one-shot effect. No test touches a real thread, a device or a
network.

## Stack

Gradle **9.6.0** · AGP **9.5.1** (built-in Kotlin: do **not** apply
`org.jetbrains.kotlin.android`) · Kotlin **2.4.21** · Compose BOM
**2026.09.00** · Navigation3 **1.2.1** (`navigation3-runtime` +
`navigation3-ui`) · lifecycle **2.11.0** · activity-compose **1.13.0** ·
coroutines **1.11.0** · serialization **1.9.0** · JUnit **4.13.2** ·
`compileSdk`/`targetSdk` **37** · `minSdk` **26** · JVM target **17**.

## Gates

`make ci` (check-architecture + `test` + `testDebugUnitTest` +
`:app:lintDebug` + `:app:assembleDebug`) · `./gradlew :evals:test` ·
`bash scripts/verify-template.sh` (template factory only: it stages a renamed
copy and runs every gate on it).

## Where the contracts live

`PLAYBOOK_MOBILE_ANDROID_KOTLIN.md` (architecture, Gradle graph, UDF,
Navigation 3, platform integration) · `PLAYBOOK_TESTS.md` ·
`PLAYBOOK_SECURITY.md` · `PLAYBOOK_FRONTEND.md` · `PLAYBOOK_URLS.md` ·
`PLAYBOOK_AUDIT.md` · `PLAYBOOK_CLAUDE_CODE.md` · `AGENTS.md` (agent
workflow) · `docs/dependency-register.md` (every dependency and its removal
path).
