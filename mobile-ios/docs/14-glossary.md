# Glossary

Every term used in this set, defined once. Where a term has an Apple meaning
and a kit meaning, the kit meaning is the one that counts here.

**@Observable** — The Observation macro that makes a class's properties
observable by the SwiftUI runtime, with no `@Published` wrapper and no
`ObservableObject`. `NotesModel` and `Router` use it. The macro does not make
a type `Sendable`; isolation is a separate decision.

**Adapter** — An implementation of a port that talks to the outside world.
`InMemoryNoteRepository` in `Packages/Feature/Notes/Data` implements
`NoteRepository`.

**App Attest** — Apple's device attestation service: the app signs a
challenge and the server verifies the assertion. A **risk signal**, evaluated
server-side — never a client-side block
([PLAYBOOK_MOBILE_IOS_SWIFT.md](../PLAYBOOK_MOBILE_IOS_SWIFT.md) §15).

**APNs** — Apple Push Notification service. The push pipeline of this kit is
**not shipped**: no registration, no token lifecycle, no categories
([10](10-push-notifications.md)).

**AppEnvironment** — The composition root. A `@MainActor` struct in the app
target that holds the model factories and builds the object graph by hand
([ADR 0003](adr/0003-explicit-app-environment.md)).

**Architecture check** — `scripts/check-architecture.sh`. The gate that
asserts the package graph: no forbidden edge, no UI framework in a pure
package, no tracked `.xcodeproj`, Swift 6 mode everywhere.

**ATS** — App Transport Security. The iOS network policy that requires TLS.
The template grants no exception: `project.yml` declares no
`NSAllowsArbitraryLoads`, and the contract evals assert it stays that way.

**AsyncStream** — Swift's stream of values. In this kit it carries the
one-shot `NotesEffect` out of the model; the app target is the single
consumer.

**Capability** — Anything the app asks the platform for: an `Info.plist`
usage key, an entitlement, a background mode, a push entitlement, a privacy
manifest. The template declares **none**
([AGENTS.md](../AGENTS.md)).

**Composition root** — The one place that knows which implementation is live.
In this kit it is `AppEnvironment.live` in the app target; nothing below it
picks an adapter.

**Contract eval** — A test that validates the `.claude` layer — the agents,
the skills, the commands — against `.claude/schemas/`, plus the package
graph, the `project.yml` posture and the placeholder leak. The suite is
`evals/contracts.test.mjs`, run with `node --test`
([12](12-testing-and-ci.md) §6).

**Effect** — A one-shot action that leaves the screen: navigate, show an
alert, request a permission. `NotesEffect` is delivered through an
`AsyncStream` and never becomes part of the state.

**Event** — What the user did, travelling **up** from the screen to the
model. `NotesEvent` in this kit is `refresh | open(NoteID)`.

**Fixture** — Frozen test data under `evals/fixtures/`. The `valid/` files
must pass the validator; the `invalid/` files must fail it. The fixtures are
**excluded** from the rename substitution pass, so a deliberate residual
placeholder keeps exercising the leak detector.

**Stub** — A hand-written test double that implements a port with a fixed
result, used by the Swift Testing suites. `StubRepository` in
`NotesDomainTests` and `NotesUITests`. There is no mocking framework in the
dependencies.

**Keychain** — The iOS system service that holds a secret the app can use
but not read. The template ships **no** Keychain wrapper
([09](09-security-and-keychain.md)).

**Known gap** — A part of the template that is deliberately not built yet: a
seam without the implementation. The gaps are listed in
[00 — Overview & Principles](00-overview-and-principles.md) and in the
[documentation index](README.md). A known gap describes current reality — do
not copy it into new code, and do not treat it as a design.

**MainActor** — The actor that owns the UI. Every model and every view in
this kit is `@MainActor`: the state is mutated on the main actor and the
compiler enforces it.

**Module** — The compiled Swift unit named in an `import`. In this kit the
module name equals the package name and the product name: `Kernel`,
`UICore`, `NotesContracts`, `NotesDomain`, `NotesData`, `NotesUI`.

**Native gate** — The tier that needs the Apple toolchain:
`scripts/native-gates.sh` runs `swift test` per package, `xcodegen generate`
and `xcodebuild`. Off macOS, or without `xcodegen` / `swift` / `xcodebuild`
on `PATH`, it prints a visible `SKIP` and exits 0.

**NavigationStack** — The SwiftUI container that owns the navigation stack.
`RootView` passes `Router.path` to `NavigationStack(path:)`.

**navigationDestination** — The SwiftUI modifier that maps a typed route
value to a view. The app's `navigationDestination(for: NotesRoute.self)`
switch is exhaustive: a new route case breaks the build until every host
handles it.

**NoteID** — The identity of a note: a `Hashable`, `Codable`,
`RawRepresentable` value type wrapping a `String`. A typed value, so a note
id cannot be passed where another id is expected.

**Outcome** — The kernel's result type: `Outcome<Value>` is `.success(value)`
or `.failure(AppError)`. An expected failure is a **value**, not a thrown
error, so the caller must handle both branches before it can read the result.

**AppError** — The kernel's expected failure: an `ErrorCode` plus an optional
`correlationID`. It never carries a user-visible message — copy lives in the
UI layer.

**ErrorCode** — The closed set of failure codes: `offline`, `forbidden`,
`policy`, `notFound`, `conflict`, `validation`, `server`, `unknown`.

**Package** — A directory with a `Package.swift` manifest, the unit of
architecture in this kit. Six ship: `Kernel`, `UICore`, `NotesContracts`,
`NotesDomain`, `NotesData` and `NotesUI`. The manifest's dependency list is
the package's architectural contract
([ADR 0002](adr/0002-one-swiftpm-package-per-layer.md)).

**Placeholder** — A literal `<app>`, `<APP>` or `<scope>` token. The template
root does not build until `scripts/rename.sh` substitutes all three; a
residual placeholder outside `scripts/` and `evals/fixtures/` is a leak.

**Port** — A protocol the consuming package declares, so the use case depends
on the abstraction and not on the implementation. The port belongs to the
**consumer**: `NoteRepository` lives in `NotesDomain`, next to the use case
that needs it.

**Privacy manifest** — The `PrivacyInfo.xcprivacy` file that declares a
binary's API usage and required-reason APIs. The template ships **none**.

**Pure package** — A SwiftPM package whose manifest declares no UI
framework. SwiftUI and UIKit are not on its classpath, so `import SwiftUI`
there is a **compile error**, not a review comment
([ADR 0002](adr/0002-one-swiftpm-package-per-layer.md)).

**Rename** — `scripts/rename.sh <app-slug> <scope>`, the substitution pass
that turns the literal placeholders into real names. It rewrites the
**contents** of text files, renames the **files** whose basename carries a
token (`<APP>App.swift` becomes `AcmeApp.swift`), and renames the
**directories** whose names carry a token (`Apps/<APP>` becomes `Apps/Acme`);
it leaves `scripts/` and `evals/fixtures/` alone. Both arguments must match
`^[a-z][a-z0-9]*$`.

**Router** — The app-owned navigation state: a `@MainActor @Observable` class
whose `path` is `[NotesRoute]`. The path is app state; no screen owns its own
stack.

**Sendable** — The concurrency marker that says a value can cross an actor
boundary safely. Ports, value types and the state are `Sendable`; the models
are `MainActor`-isolated instead.

**Slice** — A vertical feature: contracts, domain, data and UI, one package
each. The template ships one, `notes`
([04](04-vertical-slicing.md)).

**State** — The immutable value the screen renders. `NotesViewState` carries
a `UiStatus` plus the notes; the array is never `nil` to stand in for a
missing status.

**Strict concurrency** — The Swift compiler's data-race checking.
`SWIFT_STRICT_CONCURRENCY: complete` in `project.yml` and
`swiftLanguageModes: [.v6]` in every manifest.

**Swift 6 mode** — The Swift language mode that makes the concurrency rules
errors instead of warnings. Every package declares
`swiftLanguageModes: [.v6]`; `check-architecture.sh` fails when one does not.

**Swift Testing** — Apple's open-source test framework, used by every Swift
suite in this kit: `import Testing`, `@Suite`, `@Test` and `#expect`. There
is **no** XCTest target in the tree.

**SwiftPM** — Swift's package manager. A package is a directory with a
`Package.swift` manifest; the manifest's dependency list **is** the
architectural contract.

**Target** — A build unit. In SwiftPM, a target inside a package (a library
or a test target). In Xcode, an application or library target. The app
target is the only `type: application` in `project.yml`.

**UiStatus** — The closed set of statuses every screen renders: `loading`,
`ready`, `empty`, `offline`, `forbidden`, `notFound`,
`error(correlationID:)` and `policyBlocked`. A `switch` that misses one does
not compile.

**UDF** — Unidirectional data flow: state down, events up, effects out.

**Use case** — A pure-Swift struct that applies the domain's rules.
`LoadNotes` calls the port and returns an `Outcome<[Note]>`; a cancellation
is rethrown, never converted into an `AppError`.

**XcodeGen** — The generator that reads `project.yml` and writes the
`.xcodeproj`. The manifest is the source of truth; the project file is a
build artifact and is git-ignored
([ADR 0001](adr/0001-xcodegen-generated-project.md)).

## See also

- [Overview & Principles](00-overview-and-principles.md)
- [Packages, Layers & UDF](03-packages-layers-and-udf.md)
- [Testing & CI](12-testing-and-ci.md)
- [Playbooks](13-playbooks.md)
