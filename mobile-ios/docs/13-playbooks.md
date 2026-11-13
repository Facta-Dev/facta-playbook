# Playbooks

Six repeatable recipes: the changes you will make most in this kit. Each one
lists the steps, the files touched and the gate that proves it. If a change
touches more files than listed, the architecture regressed — investigate
before proceeding.

Every recipe follows the same loop: **`/spec` → `/plan` → red test → green →
`/review` → `make ci`**. The skill that owns the shape is named in each
recipe; the gate that proves it is named at the end.

## 1. Add a screen or a field to the notes slice

The `notes` slice already exists, so this is **not** `/add-slice` — that
skill explicitly declines a change inside an existing slice. This is the
ordinary spec → test → code loop.

1. **Name the behaviour** in a spec at `docs/specs/<slug>.md`: what the user
   sees, the acceptance criteria, the non-goals.
2. **Write the failing test first.** A new field on the entity goes into
   `NotesDomainTests`; a new status on the screen goes into `NotesUITests`.
   A new screen ships with a model test covering `ready`, `empty` and one
   failure status ([12 — Testing & CI](12-testing-and-ci.md)).
3. **Change the entity** in
   `Packages/Feature/Notes/Domain/Sources/NotesDomain/Note.swift`. The
   domain owns the rule; the screen does not re-implement it.
4. **Change the model** in
   `Packages/Feature/Notes/UI/Sources/NotesUI/NotesModel.swift`: the view
   state, the event that carries the action, and the effect that leaves.
5. **Change the view** in
   `Packages/Feature/Notes/UI/Sources/NotesUI/NotesScreen.swift`. It is a
   pure function of `model.state`; a business rule in a view is a review
   finding.
6. **If the screen is reachable from another screen**, it also needs a typed
   route — go to recipe 3.
7. **If the field is a new user-visible string**, it is a Swift literal in
   the view. The template ships no localization catalog.

**Files**: `Packages/Feature/Notes/Domain/Sources/`,
`Packages/Feature/Notes/UI/Sources/`,
`Packages/Feature/Notes/Domain/Tests/`,
`Packages/Feature/Notes/UI/Tests/`.

**Gate**: `make ci`, with `swift test` green in the two packages the change
touches.

**Skill**: none owns this shape — `/spec`, `/plan` and `/review` cover it.
`/add-slice` is for a **new** slice.

## 2. Add a package

**Skill**: `/add-package`.

1. **Choose the kind**: pure Swift, adapter, or UI. The kind is the
   architectural decision: a pure package has no SwiftUI or UIKit in its
   manifest, and that is the point
   ([ADR 0002](adr/0002-one-swiftpm-package-per-layer.md)).
2. **Create the directory** under `Packages/Core/` (shared, no feature) or
   `Packages/Feature/<slice>/` (owned by one slice).
3. **Write the manifest.** It declares `platforms: [.iOS(.v18),
   .macOS(.v15)]`, `swiftLanguageModes: [.v6]`, and only inward
   dependencies. The product name equals the target name equals the
   directory name.
4. **Ship the test target with the package.** A `testTarget` is part of the
   package, not an afterthought in the app.
5. **Register it** in `project.yml` under `packages:`, and in the app
   target's `dependencies:` only when the app uses it.
6. **Add the edge to the architecture check** if the package is pure:
   `scripts/check-architecture.sh` enumerates the pure packages by path, so
   a new pure package needs its own rule.
7. **Update the dependency register** if the package pulls a remote
   dependency.

**Files**: the new package directory, `project.yml`,
`scripts/check-architecture.sh`, `docs/dependency-register.md`.

**Gate**: `swift test` in the new package (macOS),
`bash scripts/check-architecture.sh`, `make ci`.

## 3. Add a route

**Skill**: `/add-route`.

1. **Declare the route** as a case of the slice's route enum in
   `Packages/Feature/Notes/Contracts/Sources/NotesContracts/NotesRoute.swift`.
   No path string, ever.
2. **Use typed payloads.** `NotesRoute.detail(id: String)` is the shipped
   shape; a payload that the domain types (`NoteID`) travels as that type.
3. **Add the destination** in the app's `RootView.swift`. The
   `navigationDestination(for:)` switch is exhaustive: a new case breaks the
   build until every host handles it.
4. **Update the deep-link parser** if one exists — it produces a route
   value and rejects an unknown path. The template ships **no** parser
   ([06](06-navigation-routes-and-deep-links.md)).
5. **Add the tests**: the route round-trips through `Hashable` equality, and
   the parser rejects an unknown path.

**Files**: `Packages/Feature/Notes/Contracts/Sources/`,
`Apps/<APP>/Sources/<APP>/RootView.swift`, the route test.

**Gate**: `bash scripts/check-architecture.sh`, `make ci`.

## 4. Add a port

**Skill**: `/add-port`.

1. **Declare the port** in the slice's `Domain` package, in terms of the
   **domain types** — never a DTO, never a persistence type, never a
   framework type. The protocol is `Sendable` and its methods are `async
   throws`.
2. **Write the use case** against the port, and the failing test against a
   stub. The use case catches `AppError` and maps an unknown error to
   `.unknown`.
3. **Implement the adapter** in the slice's `Data` package. This is the
   only place the framework types appear, and the adapter imports no UI
   framework.
4. **Write the stub** next to the test: a struct implementing the port with
   a fixed result. No mocking framework.
5. **Wire it in the app target**: `AppEnvironment.live` constructs the
   adapter and hands it to the use case. Nothing else changes
   ([ADR 0003](adr/0003-explicit-app-environment.md)).
6. **Update the dependency register** if the adapter brought a new
   dependency.

**Files**: `Packages/Feature/Notes/Domain/Sources/`,
`Packages/Feature/Notes/Data/Sources/`,
`Packages/Feature/Notes/Domain/Tests/`,
`Apps/<APP>/Sources/<APP>/AppEnvironment.swift`.

**Gate**: `make ci`, with `swift test` green in `NotesDomain`.

## 5. Replace the in-memory adapter

The template ships `InMemoryNoteRepository`: two fixed notes, no disk, no
network. Replacing it is a port-shaped change, so **`/add-port`** is the
skill that owns it — the port already exists, so step 1 of that recipe is
skipped and the protocol is extended instead of declared.

1. **Extend the port** in
   `Packages/Feature/Notes/Domain/Sources/NotesDomain/NoteRepository.swift`
   with the operations the real store needs. Domain types only.
2. **Write the failing use-case test** against the stub, with the new
   operations exercised.
3. **Implement the real adapter** in
   `Packages/Feature/Notes/Data/Sources/NotesData/`. The persistence
   technology lives here and nowhere else.
4. **Write the adapter's own test** against the store's own test facilities —
   an in-memory database, a stubbed transport — not against a live server.
   `NotesData` ships no test target today; adding one is part of this
   change.
5. **Swap the wiring** in `Apps/<APP>/Sources/<APP>/AppEnvironment.swift`:
   one line changes from `InMemoryNoteRepository()` to the new adapter.
   The screens do not change; the model does not change; the use case does
   not change.
6. **Delete `InMemoryNoteRepository`** once nothing references it, or keep
   it as the test double — but do not ship two live implementations with no
   rule saying which is live.
7. **Add a dependency-register entry** for the new persistence or network
   dependency, with the path that removes it.

**Files**: `Packages/Feature/Notes/Domain/Sources/`,
`Packages/Feature/Notes/Data/Sources/`,
`Packages/Feature/Notes/Domain/Tests/`,
`Apps/<APP>/Sources/<APP>/AppEnvironment.swift`,
`docs/dependency-register.md`.

**Gate**: `make ci`, with `swift test` green in `NotesDomain` and the new
adapter's own package.

## 6. Add a capability

A capability is anything the app asks the platform for: an `Info.plist` usage
key, an entitlement, a background mode, a push entitlement, a privacy
manifest. The template declares **none** of them.

**No skill owns this shape.** The rules come from
[AGENTS.md](../AGENTS.md) and [PLAYBOOK_SECURITY.md](../PLAYBOOK_SECURITY.md),
and the review note is mandatory.

1. **Write the spec** with the capability, the justification and the
   user-visible behaviour it enables.
2. **Plan it as its own step.** `/plan` gives a capability change its own
   phase, with the review note, separate from the code that uses it.
3. **Declare it in `project.yml`**, never in Xcode's UI: the
   `INFOPLIST_KEY_*` settings for usage keys, an entitlements file, the
   background modes. The project file is generated and git-ignored
   ([ADR 0001](adr/0001-xcodegen-generated-project.md)).
4. **Write the test**: the denial path is a first-class case, not an
   afterthought.
5. **Get the security review**: the `ios-security-reviewer` agent, against
   [PLAYBOOK_SECURITY.md](../PLAYBOOK_SECURITY.md).
6. **Update the dependency register** with the capability entry.

**Files**: `project.yml`, the entitlements or privacy manifest file, the
test, `docs/dependency-register.md`.

**Gate**: `make ci` — `check-architecture.sh` fails if a `.xcodeproj` is
tracked, and the contract evals check that `project.yml` declares no
credential and no blanket transport grant.

## The pre-review checklist

- [ ] The domain owns the rule; the screen does not re-implement it.
- [ ] The port speaks domain types and the use case returns `Outcome<Value>`.
- [ ] The model is `@MainActor @Observable`; the state is immutable.
- [ ] The effect leaves once, through the `AsyncStream`.
- [ ] The route is a typed case and the `navigationDestination` is exhaustive.
- [ ] The new package is in `project.yml`, in its `Package.swift` and in the
      architecture check.
- [ ] Every package declares `swiftLanguageModes: [.v6]`.
- [ ] The new behaviour has a test against a stub.
- [ ] The dependency register is updated.
- [ ] `bash scripts/check-architecture.sh` and the whole gate set are green.
- [ ] The native tier's `SKIP` is reported as a `SKIP`.

## Do / don't

| Do | Don't |
| --- | --- |
| Follow the recipe; a longer diff means the architecture changed | Add a shortcut "just this once" |
| Declare the port in the domain | Declare the port in the adapter |
| Register the route in the app target | Let the screen register itself |
| Update the register in the same change | Change a dependency without a register row |
| Keep the architecture check green | Disable the check to make a build pass |
| Report the native `SKIP` honestly | Report a build that never ran |

## See also

- [Vertical Slicing](04-vertical-slicing.md) — the shape of a slice.
- [Packages, Layers & UDF](03-packages-layers-and-udf.md) — the package
  graph.
- [Testing & CI](12-testing-and-ci.md) — the gates.
- [Security & Keychain](09-security-and-keychain.md) — the capability rules.
- [Admin Guide](admin-guide.md) — signing and App Store Connect.
- [ADR 0002 — One SwiftPM package per layer](adr/0002-one-swiftpm-package-per-layer.md)
- [ADR 0003 — Explicit AppEnvironment](adr/0003-explicit-app-environment.md)
