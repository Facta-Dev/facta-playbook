# Playbooks

Six repeatable recipes: the changes you will make most. Each one lists the
steps, the files touched and the gate that proves it. If a change touches
more files than listed, the architecture regressed — investigate before
proceeding.

## 1. Add a slice (a new feature)

1. **Create the four modules** under `feature/<slice>/`: `contracts`,
   `domain`, `data`, `ui`. Register them in `settings.gradle.kts`.
2. **Domain first.** The entity and its invariants, the port, the use cases.
   Pure Kotlin: no `android.*`, no Compose. Write the failing test first.
3. **Contracts.** The `@Serializable` `NavKey` route types, and any
   interface another slice may call.
4. **Data.** The adapter implementing the port. This is where Room, the SDK
   or a cache goes when they are added.
5. **Ui.** The ViewModel, the state, the events, the effects and the
   screen. The slice names them after itself: `NotesState`, `NotesEvent`,
   `NotesEffect`.
6. **Register the route** in `:app`, in the `entryProvider`. It is the only
   place a route is paired with a screen.
7. **Wire the implementations** in the `:app` composition root. This is
   the only place that chooses an implementation.
8. **Add the route to the route table** and update the parity test
   ([06](06-navigation-routes-and-deep-links.md)). The table is a
   cross-kit contract; until it exists, this step waits with it.
9. **Add the tests**: the use cases against a fake, the ViewModel against
   the use cases.

**Files**: `feature/<slice>/**`, `settings.gradle.kts`, `app/` (the
`entryProvider` and the wiring), the route table.

**Gate**: `bash scripts/check-architecture.sh && ./gradlew test
./gradlew testDebugUnitTest ./gradlew :app:assembleDebug`.

## 2. Add a module

1. **Choose the kind**: pure JVM (`kotlin("jvm")`) or Android library
   (`com.android.library`). The kind is the architectural decision: a pure
   JVM module cannot import `android.*`, and that is the point.
2. **Create the directory** and the build file. Declare the plugins with
   `alias(libs.plugins.…)`; never with a version.
3. **Register it** in `settings.gradle.kts`.
4. **Declare the dependencies** with `alias(libs.…)` and nothing else. The
   dependency list *is* the module's architectural contract.
5. **Add the source sets**: `src/main/kotlin` and `src/test/kotlin`.
6. **Add the module to the architecture check**, if it is a kind the
   script does not already cover. The script enumerates the modules it
   knows by path: a new module of an existing kind is checked only if it is
   added to those lists. A new **kind** of module always needs a new rule.

**Files**: the new directory, `settings.gradle.kts`, the version catalog if a
dependency is new, `docs/dependency-register.md`.

**Gate**: `bash scripts/check-architecture.sh && ./gradlew
:feature:<slice>:<new>:test`.

## 3. Add a route

1. **Declare the route** in the slice's `contracts` module as a
   `@Serializable` data class implementing `NavKey`. No path string.
2. **Add the screen** in the slice's `ui` module.
3. **Register the pair** in `:app`'s `entryProvider`.
4. **Add the route to the shared route table** and update the parity test.
   The table is a cross-kit contract; until it exists, this step waits
   with it ([06](06-navigation-routes-and-deep-links.md)).
5. **If it is reachable from outside** — an App Link or a notification —
   add it to the deep-link allowlist and to the parser's validation
   ([06](06-navigation-routes-and-deep-links.md)).
6. **Add the test**: the screen per status, and the parser test if it is
   externally reachable.

**Files**: `feature/<slice>/contracts/`, `feature/<slice>/ui/`, `app/`, the
route table.

**Gate**: `./gradlew test ./gradlew testDebugUnitTest ./gradlew
:app:assembleDebug`.

## 4. Add a port and its adapter

1. **Declare the port** in the slice's `domain` module, in terms of the
   **domain types** — never a DTO, never a Room entity, never a framework
   type. Every method returns `Outcome<T>`.
2. **Write the use case** against the port, and the failing test against a
   fake.
3. **Implement the adapter** in the slice's `data` module. This is the only
   place the framework types appear.
4. **Write the adapter's test** against the framework's own test facilities
   — an in-memory Room database, a `MockWebServer` — not against a live
   server.
5. **Wire it in `:app`**: the composition root builds the adapter and
   passes it to the use case. Nothing else changes.
6. **Update the dependency register** if the adapter brought a new
   dependency.

**Files**: `feature/<slice>/domain/`, `feature/<slice>/data/`, `app/`.

**Gate**: `./gradlew test ./gradlew testDebugUnitTest`.

## 5. Change a dependency

1. **Read the register** ([Dependency Register](dependency-register.md)):
   what the dependency is for, and what it would cost to remove.
2. **Change the version in `gradle/libs.versions.toml`**, never in a
   `dependencies` block.
3. **Update the register** in the same change: the new version, and the
   date.
4. **Run the whole gate set**, including `:app:assembleDebug` — a
   dependency bump that breaks the build is only visible at compile time.
5. **Check the removal path**: if the dependency became unnecessary, remove
   it in this change, and record that in the register.
6. **Check the permission and the data-safety impact**: a new SDK may
   declare a permission or collect data. If it does, the Play data safety
   form changes too ([09](09-security-and-keystore.md)).

**Files**: `gradle/libs.versions.toml`, `docs/dependency-register.md`, and
the module that declares the dependency.

**Gate**: `bash scripts/check-architecture.sh && ./gradlew
:app:assembleDebug ./gradlew test ./gradlew testDebugUnitTest`.

## 6. Ship a release

1. **The gate set is green**: `bash scripts/check-architecture.sh &&
   ./gradlew :app:assembleRelease ./gradlew test ./gradlew testDebugUnitTest
   ./gradlew :evals:test`.
2. **Configure the release build**: R8 full mode, resource shrinking,
   `isDebuggable = false`, the keep rules per library, and the signing
   config ([09](09-security-and-keystore.md)).
3. **Build the AAB**: `./gradlew :app:bundleRelease`. The Play App Signing
   key is held by the Play Console; the **upload key** lives in the CI
   secret store, never on a developer machine
   ([Admin Guide](admin-guide.md)).
4. **Verify the manifest**: the merged manifest, the permission set and the
   native libraries, against the register. A permission that is not in the
   register does not ship.
5. **Run the release checks on the Low pool**: the startup and memory
   budgets ([02](02-device-tiers-and-the-quality-governor.md)).
6. **Upload to the internal track**, read the pre-launch report, then
   promote: internal → closed → production, with a staged rollout.
7. **Update the version**: the `versionCode` and the `versionName` in
   `:app`. The template ships no diagnostics bundle — the release notes and
   the store listing carry the same version string.
8. **Archive the evidence**: the build, the test reports, the permission
   diff, the approval.

**Files**: `app/build.gradle.kts` (the release config and the version),
`docs/dependency-register.md`, the Play Console.

**Gate**: the whole set of step 1, plus the store-readiness checks.

## The pre-review checklist

- [ ] The domain owns the rule; the screen does not re-implement it.
- [ ] The port speaks domain types and returns `Outcome<T>`.
- [ ] Nothing blocks the main thread; the dispatcher is injected.
- [ ] No secret, token or note body reaches a log.
- [ ] No secret is in `BuildConfig`.
- [ ] The route is typed and registered in `:app`.
- [ ] The new module is in `settings.gradle.kts` and in the architecture
      check.
- [ ] The new behaviour has a test against the fake.
- [ ] The dependency register is updated.
- [ ] `bash scripts/check-architecture.sh` and the whole gate set are green.

## Do / don't

| Do | Don't |
| --- | --- |
| Follow the recipe; a longer diff means the architecture changed | Add a shortcut "just this once" |
| Declare the port in the domain | Declare the port in the adapter |
| Register the route in `:app` | Let the screen register itself |
| Update the register in the same change | Change a dependency without a register row |
| Keep the architecture check green | Disable the check to make a build pass |

## See also

- [Vertical Slicing](04-vertical-slicing.md) — the shape of a slice.
- [Layers, Modules & UDF](03-layers-modules-and-udf.md) — the module graph.
- [Testing & CI](12-testing-and-ci.md) — the gates.
- [Admin Guide](admin-guide.md) — signing and the Play Console.
