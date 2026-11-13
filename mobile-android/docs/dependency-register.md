# Dependency Register

Every dependency the template declares, with the version, the reason it is
there, and the path that removes it. **A dependency that is not in this
register does not belong in `gradle/libs.versions.toml`.**

The register is the evidence for the least-dependency posture. A dependency
is a permanent maintenance obligation: it needs version bumps, it needs a
security watch, and it needs a removal plan.

## 1 · The build toolchain

| Dependency | Version | Why it is there | Removal path |
| --- | --- | --- | --- |
| Gradle | 9.6.0 | The build. The version is pinned by the wrapper, so every machine and CI runner uses the same one. | Bump the wrapper properties file; the Gradle upgrade guide is the removal path for an old version. |
| Android Gradle Plugin | 9.5.1 | Turns a module into an Android app or library; owns the SDK levels, the manifest merging and the packaging. | Not removable. AGP 9 compiles Kotlin itself, so the separate Kotlin Android plugin is not applied. |
| Kotlin | 2.4.21 | The language, the coroutines, and the Compose compiler plugin. The K2 compiler is fast and reflection-light, which matters for the startup budget. | Not removable. |
| `kotlinx-serialization` plugin | 2.4.21 (matched) | The `@Serializable` `NavKey` route types need it, and `:app` builds a polymorphic `SerializersModule` from the generated serializers. | Removing it removes the typed routes; that is a redesign, not a removal. |

## 2 · The runtime libraries

| Dependency | Version | Why it is there | Removal path |
| --- | --- | --- | --- |
| Compose BOM | 2026.09.00 | Pins the whole Compose artifact set to one compatible version. The BOM, not individual versions, is the unit of upgrade. | Bump the BOM. A downgrade is not supported. |
| `androidx.activity:activity-compose` | 1.13.0 | `ComponentActivity` and the `setContent` host. It is an AndroidX artifact outside the Compose BOM, so its version is in the catalog. | Replace with a plain `Activity` and a manual view tree; the app loses the Compose entry point. |
| `androidx.lifecycle:lifecycle-viewmodel-compose` | 2.11.0 | The `ViewModel`, the `viewModelScope`, and `collectAsStateWithLifecycle()` — the last one stops the collection when the UI is not active, which is a battery and CPU saving on the Low tier. | Replace with a `StateFlow` collected in a `LaunchedEffect`; the UI then keeps collecting in the background. |
| `androidx.lifecycle:lifecycle-runtime-compose` | 2.11.0 | The lifecycle-aware collection APIs, and the lifecycle that the ViewModel is scoped to. | As above. |
| `androidx.navigation3:navigation3-runtime` | 1.2.1 | The `NavKey` interface, `rememberNavBackStack` and `entryProvider` — the typed back stack and the route-to-screen mapping. | Replacing it means replacing the route model with strings — see [ADR 0002](adr/0002-navigation3-typed-navkeys.md). |
| `androidx.navigation3:navigation3-ui` | 1.2.1 | The `NavDisplay` composable that renders the back stack. It is a separate artifact from the runtime. | The back stack would need a hand-written renderer. |
| `org.jetbrains.kotlinx:kotlinx-serialization-core` | 1.9.0 | The `@Serializable` annotations on the route keys; the runtime is pulled in by the serialization plugin. | It is what makes a route a typed key. Not removable. |
| `org.jetbrains.kotlinx:kotlinx-coroutines-core` | 1.11.0 | `Outcome`-returning `suspend` functions, `Flow` for the observable state, and the dispatchers the classes inject. | The whole asynchronous model depends on it. Not removable. |
| `androidx.compose.ui:ui` | Via the BOM | The Compose runtime, the layout and the `Modifier` system. | Not removable without abandoning Compose. |
| `androidx.compose.material3:material3` | Via the BOM | The Material 3 components — the app's whole look. | Not removable; the design system is built on it. |

## 3 · The test libraries

| Dependency | Version | Why it is there | Removal path |
| --- | --- | --- | --- |
| `junit:junit` | 4.13.2 | The unit tests, in the JVM modules and in the `:evals` module. JUnit 4 because the platform's own test infrastructure and the AndroidX test rules are built on it. | Migrate to JUnit 5 with the `junit-vintage` engine; the tests are annotated the same way. |
| `org.jetbrains.kotlinx:kotlinx-coroutines-test` | 1.11.0 | `TestScope`, `UnconfinedTestDispatcher` and `runTest`: a unit test never waits on a real thread and never sleeps. | Replace with a hand-rolled dispatcher and `withTimeout`; the tests get slower and flakier. |
| `org.jetbrains.kotlinx:kotlinx-serialization-json` | 1.9.0 | `:evals` parses `.claude/settings.json` and `.claude/manifest.json`; the test build uses the JSON codec. | A hand-written parser in the test — more code, no benefit. |

## 4 · What is deliberately **not** in the template

The absences are as deliberate as the presences. Each of these is an
**adoption step** with a chapter, not an oversight.

| Not present | Why it is not there yet | The adoption path |
| --- | --- | --- |
| **Hilt** (or any DI framework) | The template has eight modules and a composition root of a handful of objects. A DI framework is a build system of its own; at this size manual DI is smaller and clearer. See [ADR 0003](adr/0003-manual-di-in-the-app-module.md). | When the graph exceeds what a human can hold, or when the team wants to test injection graphs, adopt a DI framework in `:app` only. |
| **Room** | The `notes` repository is an in-memory adapter. Room is a persistence decision the template cannot make for you. | [08 — Offline, Outbox & Sync](08-offline-outbox-and-sync.md) §7 |
| **WorkManager** | There is no background work to schedule. | [08](08-offline-outbox-and-sync.md) §7 |
| **Tink** | There is no secret to encrypt. | [09 — Security & Keystore](09-security-and-keystore.md) §9 |
| **Firebase Messaging** | There is no push pipeline. | [10 — Push Notifications](10-push-notifications.md) §10 |
| **OkHttp** | There is no network client. | [07 — Auth & Platform Features](07-auth-and-platform-features.md) §10 |
| **Coil** | The template renders no remote images. | When the first remote image appears. |
| **DataStore** | The template persists nothing. | [08](08-offline-outbox-and-sync.md) §7 |
| **MockK / Mockito** | The fakes are hand-written and small. A mocking framework for one port is not worth its weight. | When a port has more than three implementations in a test. |
| **Turbine** | A `Flow` can be asserted with a plain collector and a `TestScope`. | When the flow assertions get noisy. |

## 5 · The rules

| Rule | Why |
| --- | --- |
| One version per artifact, in the catalog | A tree-wide bump is one edit |
| **No dynamic versions** (`+`, `latest.release`) | The build must be reproducible; a build that changes under you is not a build |
| The Compose artifacts come from the BOM | They must move together |
| Every new dependency gets a **register row in the same change** | A dependency without a removal path is a dependency without an owner |
| A dependency that is not used is **removed**, not kept "for later" | An unused dependency is still an attack surface and still a license to track |
| Every dependency is checked for its **license** and its **maintenance health** | A dependency with one maintainer is a risk |
| The dependency verification metadata is generated and committed | A dependency that changes its bytes without changing its version is a supply-chain incident. The template does **not** ship it: `./gradlew --write-verification-metadata sha256` generates it, and it belongs to a project, not to a kit that a project renames |

## 6 · The review process

A dependency change is a review, in this order:

1. **Read the register.** What is the dependency for, and what would it cost
   to remove?
2. **Change the version** in `gradle/libs.versions.toml`.
3. **Update this register** in the same change: the version and the date.
4. **Run the whole gate set**, including `:app:assembleDebug` — a bump that
   breaks the build is only visible at compile time.
5. **Check the side effects**: a new SDK may declare a **permission** or
   **collect data**. If it does, the permission register and the Play data
   safety form change too ([09](09-security-and-keystore.md)).
6. **Check the removal path**: if the dependency became unnecessary, remove
   it in this change and record that in the register.

## 7 · The removal path, exercised

Every dependency has a way out, and the way out is exercised the same way:

| Step | What it proves |
| --- | --- |
| Remove the line from the catalog | The artifact is no longer resolved |
| Remove the `alias(libs.…)` from the module | No module asks for it |
| Delete the code that used it | No dead code keeps the call alive |
| Run the gate set | Nothing referenced the removed API |
| Update this register | The evidence matches the build |

## See also

- [Project Layout & Build Logic](05-project-layout-and-build-logic.md) — the
  version catalog.
- [Security & Keystore](09-security-and-keystore.md) — the permission and
  data-safety side effects.
- [Playbooks](13-playbooks.md) — the recipe for changing a dependency.
