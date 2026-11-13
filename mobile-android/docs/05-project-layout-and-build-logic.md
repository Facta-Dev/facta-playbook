# Project Layout & Build Logic

The tree, the build files, the version catalog and the command that proves
it all compiles.

## 1. The tree

```text
mobile-android/
├── settings.gradle.kts          # the module list and the plugin/dependency repositories
├── build.gradle.kts             # the root build: plugins declared, not applied
├── gradle.properties            # JVM args, AndroidX, non-transitive R class
├── gradle/
│   ├── libs.versions.toml       # the version catalog — the single source of versions
│   └── wrapper/                 # the Gradle wrapper
├── gradlew · gradlew.bat        # the wrapper scripts
├── app/                         # :app — the composition root, the manifest, NavDisplay
├── core/
│   ├── kernel/                  # :core:kernel — Outcome, ids, errors (pure JVM)
│   └── ui-core/                 # :core:ui-core — UiStatus, the closed status set
├── feature/
│   └── notes/
│       ├── contracts/           # :feature:notes:contracts — NavKey routes (pure JVM)
│       ├── domain/              # :feature:notes:domain — entity, ports, use cases (pure JVM)
│       ├── data/                # :feature:notes:data — the repository adapter
│       └── ui/                  # :feature:notes:ui — the Compose screen and its ViewModel
│           └── res/values/strings.xml  # the slice's own user-visible strings
├── evals/                       # :evals — the JUnit contract tests for the .claude layer
├── scripts/
│   ├── check-architecture.sh    # the architecture gate
│   ├── rename.sh                # the three-token substitution
│   ├── jdk.sh                   # pins a JDK with javac for the build
│   ├── verify-template.sh       # runs every gate on a renamed copy under /tmp
│   ├── install-claude-runtime.sh # installs the kit's hooks and settings
│   ├── instruction-budget.sh    # the instruction budget of the kit's own files
│   ├── leak-guard.sh            # the placeholder and secret sweep
│   ├── lint-agents.sh           # the agents linter
│   ├── lint-commands.sh         # the commands linter
│   ├── lint-skills.sh           # the skills linter
│   └── hooks/guard-bash.sh      # the PreToolUse guard
├── .claude/                     # the agent kit: agents, skills, commands, schemas
├── docs/                        # this set
├── AGENTS.md                    # the kit's agent instructions
├── MEMORY.md                    # the kit's memory layer
├── CLAUDE.md -> AGENTS.md       # the symlink for the Claude Code runtime
├── README.md                    # the kit's front door
├── Makefile                     # make help/test/evals/build/lint/clean/verify/ci
├── .env.example                 # the shape of the environment file
└── PLAYBOOK_*.md                # the seven playbooks the kit ships
```

The layout is a **direct image of the module graph**
([03](03-layers-modules-and-udf.md)): `core/` holds the shared modules,
`feature/` holds the slices, `app/` holds the composition root, `evals/`
holds the `.claude` contract tests.

## 2. The module list

`settings.gradle.kts` names every module. Adding a module means adding it
there and nowhere else:

```kotlin
include(":app")
include(":core:kernel")
include(":core:ui-core")
include(":feature:notes:contracts")
include(":feature:notes:domain")
include(":feature:notes:data")
include(":feature:notes:ui")
include(":evals")
```

The Gradle project path and the directory path are not the same string, and
that is deliberate: the project path is the **stable identity** used by every
`dependencies` block, `tasks --all` invocation and CI command, while the
directory is a layout detail that may change.

| Gradle path | Directory |
| --- | --- |
| `:app` | `app/` |
| `:core:kernel` | `core/kernel/` |
| `:core:ui-core` | `core/ui-core/` |
| `:feature:notes:contracts` | `feature/notes/contracts/` |
| `:feature:notes:domain` | `feature/notes/domain/` |
| `:feature:notes:data` | `feature/notes/data/` |
| `:feature:notes:ui` | `feature/notes/ui/` |
| `:evals` | `evals/` |

## 3. The module kinds

Two kinds of module, and the difference is the whole architecture.

| Kind | Which | Plugin | Consequence |
| --- | --- | --- | --- |
| **Pure JVM** | `:core:kernel`, `:feature:notes:contracts`, `:feature:notes:domain`, `:evals` | `kotlin("jvm")` | **The Android SDK is not on the classpath.** An `import android.*` does not compile. |
| **Android library** | `:core:ui-core`, `:feature:notes:data`, `:feature:notes:ui` | `com.android.library` | Android APIs are available; the module has no launcher, no `Application` |
| **Android application** | `:app` | `com.android.application` | The only module with a manifest entry, a launcher icon and an `applicationId` |

The pure JVM modules are what make
[ADR 0001](adr/0001-one-gradle-module-per-layer.md) true rather than
aspirational.

### The namespaces

Every Android module declares a **namespace**, and it is unique across the
build. The namespaces are rooted at `dev.<scope>.<app>`, the same root the
`.claude` layer uses, so the application id, the namespaces and the
generated code all share one prefix:

| Module | Namespace |
| --- | --- |
| `:app` | `dev.<scope>.<app>` |
| `:core:ui-core` | `dev.<scope>.<app>.core.uicore` |
| `:feature:notes:data` | `dev.<scope>.<app>.feature.notes.data` |
| `:feature:notes:ui` | `dev.<scope>.<app>.feature.notes.ui` |
| A pure JVM module | **No namespace** — it has no Android manifest |

The `applicationId` of `:app` is `dev.<scope>.<app>`; it is the id the
store knows the app by, and it must match the app id in the Play Console
([Admin Guide](admin-guide.md) §4).

The `<scope>` and `<app>` tokens are substituted by `scripts/rename.sh`
along with the rest of the tree, so the namespace follows the rename.

## 4. The version catalog

Every version in the build comes from `gradle/libs.versions.toml`. No module
declares a version inline — a version in a `dependencies` block is a review
finding.

```toml
[versions]
agp = "9.5.1"
kotlin = "2.4.21"
composeBom = "2026.09.00"
activityCompose = "1.13.0"
lifecycle = "2.11.0"
navigation3 = "1.2.1"
coroutines = "1.11.0"
serialization = "1.9.0"
junit = "4.13.2"

[libraries]
compose-bom = { module = "androidx.compose:compose-bom", version.ref = "composeBom" }
compose-ui = { module = "androidx.compose.ui:ui" }
compose-material3 = { module = "androidx.compose.material3:material3" }
activity-compose = { module = "androidx.activity:activity-compose", version.ref = "activityCompose" }
lifecycle-viewmodel-compose = { module = "androidx.lifecycle:lifecycle-viewmodel-compose", version.ref = "lifecycle" }
lifecycle-runtime-compose = { module = "androidx.lifecycle:lifecycle-runtime-compose", version.ref = "lifecycle" }
navigation3-runtime = { module = "androidx.navigation3:navigation3-runtime", version.ref = "navigation3" }
navigation3-ui = { module = "androidx.navigation3:navigation3-ui", version.ref = "navigation3" }
kotlinx-coroutines-core = { module = "org.jetbrains.kotlinx:kotlinx-coroutines-core", version.ref = "coroutines" }
kotlinx-coroutines-test = { module = "org.jetbrains.kotlinx:kotlinx-coroutines-test", version.ref = "coroutines" }
kotlinx-serialization-core = { module = "org.jetbrains.kotlinx:kotlinx-serialization-core", version.ref = "serialization" }
kotlinx-serialization-json = { module = "org.jetbrains.kotlinx:kotlinx-serialization-json", version.ref = "serialization" }
junit = { module = "junit:junit", version.ref = "junit" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
android-library = { id = "com.android.library", version.ref = "agp" }
kotlin-jvm = { id = "org.jetbrains.kotlin.jvm", version.ref = "kotlin" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
kotlin-serialization = { id = "org.jetbrains.kotlin.plugin.serialization", version.ref = "kotlin" }
```

Note what is **not** in the plugin list: there is no `kotlin-android`.
AGP 9 builds the Kotlin sources itself, so applying
`org.jetbrains.kotlin.android` on top of it is wrong.

The catalog is the file the dependency register refers to
([Dependency Register](dependency-register.md)). Adding a dependency means
adding a line here **and** a row in the register, in the same change.

| Rule | Why |
| --- | --- |
| One place per version | A tree-wide bump is one edit |
| No dynamic versions (`+`, `latest.release`) | The build must be reproducible |
| The Compose BOM pins the Compose artifacts | Compose's own artifacts must move together |
| The plugin versions live in the catalog too | The AGP and the Kotlin plugin must agree |

## 5. The SDK levels and the toolchain

| Setting | Value | Where |
| --- | --- | --- |
| `compileSdk` | 37 | Every Android module |
| `targetSdk` | 37 | `:app` only — the library modules inherit it from the app |
| `minSdk` | 26 | Every Android module |
| JVM target | 17 | Every module, pure JVM and Android alike |
| Gradle | 9.6.0 | The wrapper |
| AGP | 9.5.1 | The catalog |
| Kotlin | 2.4.21 | The catalog |

`minSdk` 26 (Android 8.0) is the floor: it is above the AndroidX floor of
24 and gives the template the notification channels
(`NotificationChannel`, API 26+) and the modern `PendingIntent` flags
without a version check on every call site.

The JVM target of 17 applies to **every** module, including the Android
ones: AGP compiles the Java and the Kotlin sources of a module to the same
target, and a mismatch produces the confusing
`Inconsistent JVM-target compatibility` failure.

## 6. The commands

Every command below is a real Gradle task.

| Command | What it does |
| --- | --- |
| `./gradlew :app:assembleDebug` | Builds the debug APK — the compile proof of the whole tree |
| `./gradlew test` | The unit tests of every **pure JVM** module (`:core:kernel`, `:feature:notes:contracts`, `:feature:notes:domain`, `:evals`) |
| `./gradlew testDebugUnitTest` | The unit tests of every **Android** module, in the debug variant |
| `./gradlew :evals:test` | The `.claude` contract tests, named explicitly |
| `./gradlew :feature:notes:domain:test` | The unit tests of one pure JVM module |
| `./gradlew :feature:notes:ui:testDebugUnitTest` | The unit tests of one Android module |
| `./gradlew :app:assembleRelease` | The release build (unsigned, unless a signing config is added) |
| `./gradlew :app:lintDebug` | Android Lint |
| `./gradlew projects` | The module tree, as Gradle sees it |
| `./gradlew tasks --all` | Every task in every module |
| `bash scripts/check-architecture.sh` | The architecture gate |

The `Makefile` wraps the same gates in shorter names, so a CI job and a
developer run the same commands:

| Command | What it does |
| --- | --- |
| `make test` | `./gradlew test testDebugUnitTest` |
| `make evals` | `./gradlew :evals:test` |
| `make build` | `./gradlew :app:assembleDebug` |
| `make lint` | `./gradlew :app:lintDebug` |
| `make verify` | `bash scripts/verify-template.sh` — stages a renamed copy under `/tmp` and runs every gate on it |
| `make ci` | `bash scripts/check-architecture.sh` then `./gradlew test testDebugUnitTest :app:lintDebug :app:assembleDebug` |
| `make clean` | `./gradlew clean` |

Two Gradle facts worth knowing, because they are the usual cause of a
confusing failure:

- A pure JVM module's test task is `test`; an Android module's is
  `testDebugUnitTest` (and `testReleaseUnitTest`). `./gradlew test` runs the
  former set; the latter needs its own task name.
- `./gradlew test` at the root runs `test` in **every** module that has
  one, including `:evals`. `./gradlew :evals:test` runs only that module.

## 7. The gate

```sh
bash scripts/check-architecture.sh   # the module graph
./gradlew :app:assembleDebug         # it compiles
./gradlew test                       # the pure JVM unit tests
./gradlew testDebugUnitTest          # the Android unit tests
./gradlew :app:lintDebug             # Android Lint
./gradlew :evals:test                # the .claude contract tests
```

`make ci` runs the first five in one go: the architecture check, both test
suites, Android Lint and the debug build. A linter the kit ships is a
linter the gate runs — otherwise it is a linter nobody reads.

The order matters only for the reader: the architecture check is the
cheapest and catches the mistake that would waste the rest of the run.

`bash scripts/verify-template.sh` is the gate above the gate: it copies the
kit to a temporary directory, runs `scripts/rename.sh` on the copy, and runs
the whole gate set on the renamed result. That is what proves the
substitution contract — that the kit still compiles after it stops being a
template.

## 8. The convention-plugin adoption path

The template **does not ship** a `build-logic` included build. Each module
declares its own plugin block, which is verbose but obvious.

That verbosity is the seam. The adoption path, when the module count grows
past a dozen:

1. **Create `build-logic/`** as an included build:
   `settings.gradle.kts` gets `includeBuild("build-logic")`.
2. **Add one convention plugin per module kind**: `android.application`,
   `android.library`, `kotlin.jvm`, `android.compose`, `android.library.compose`.
3. **Move the repeated configuration** into the plugins: the SDK levels, the
   JVM target, the Compose compiler, the lint config, the test options.
4. **Replace each module's `plugins` block** with the convention plugin id:
   ```kotlin
   plugins {
       id("<app>.android.library")
       id("<app>.android.library.compose")
   }
   ```
5. **Keep the version catalog as the single source of versions.** The
   convention plugins read it; they do not restate it.

| What the convention plugin buys | What it costs |
| --- | --- |
| One place to change `minSdk` for the whole tree | An included build to configure and debug |
| One place to turn on a lint rule set | One more layer between the module and the plugin it uses |
| Consistency that a reviewer does not have to check | A plugin that is out of date silently changes every module |

Do not adopt it before the repetition is real. Eight modules with three
distinct shapes do not need a build system of their own.

## 9. Starting a project from the template

```sh
cp -a mobile-android/ my-app/        # copy, then work in the copy
cd my-app/
bash scripts/rename.sh <app-slug> <org-scope>
./gradlew :app:assembleDebug
./gradlew test
```

`scripts/rename.sh` substitutes the three placeholder tokens across every
text file, and the renamed copy is where the build happens — the template
root keeps the literal tokens so the kit stays a template.

The script takes the new app slug first and the new organisation scope
second:

```sh
bash scripts/rename.sh <new-app-slug> <new-org-scope>
```

Both values become Java package segments (`dev.<scope>.<app>`), so the
script accepts only a lowercase identifier — a letter first, then letters,
digits or underscore — and rejects anything else **before** it touches the
tree. A hyphenated slug would leave an unbuildable namespace behind
(`Namespace 'dev.acme.my-app' is not a valid Java package name`), and a
slash would abort the substitution half way through the copy.

Two paths are **excluded** from the substitution: `scripts/*` and
`evals/fixtures/*`. The scripts keep their literal tokens because they are
the tools that perform the substitution, and the fixtures are the
before-and-after samples of it.

The template ships no lockfile to commit; the Gradle wrapper properties file
is the pinned toolchain, and it is the file that must be reviewed when the
Gradle version changes.

## Do / don't

| Do | Don't |
| --- | --- |
| Declare every version in the catalog | Put a version in a `dependencies` block |
| Run `./gradlew test` for the JVM modules | Assume `test` covers the Android modules |
| Add a module to `settings.gradle.kts` | Leave a module out of the include list |
| Keep the JVM target at 17 everywhere | Let one module drift to a different target |
| Adopt convention plugins when the repetition is real | Add a build system for eight modules |

## See also

- [Layers, Modules & UDF](03-layers-modules-and-udf.md) — what each module
  may depend on.
- [Testing & CI](12-testing-and-ci.md) — the gates in detail.
- [Dependency Register](dependency-register.md) — every dependency and its
  removal path.
