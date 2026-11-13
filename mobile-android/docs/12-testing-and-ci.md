# Testing & CI

The gates are the contract. A change is finished when the gate set is green;
a change to the kit's own agent runtime is finished when the evals pass.

## 1. The test tiers

| Tier | Where it runs | Tooling | What it covers |
| --- | --- | --- | --- |
| **Unit** | JVM, `./gradlew test` | JUnit 4, coroutines-test | The kernel, the domain, the use cases |
| **Unit (Android)** | JVM, `./gradlew testDebugUnitTest` | JUnit 4, coroutines-test | The ViewModels, the mappers, the adapters |
| **Architecture** | The shell | `scripts/check-architecture.sh` | The module graph |
| **Contract** | JVM, `./gradlew :evals:test` | JUnit 4 | The `.claude` layer: agents, skills, commands, manifest posture, placeholder leaks |
| **Instrumented** | A device or an emulator | AndroidX Test, Compose UI test | The screens, the real Android APIs, the Keystore |
| **E2E** | A device or an emulator | UI Automator / Maestro | The user journeys |

The template ships the three JVM tiers and the architecture gate; the
**instrumented** and **E2E** tiers are adoption steps — the layout in §3 is
where they go, not tests that already exist.

The split between the unit tiers is the module split: a test in a pure JVM
module runs under `./gradlew test`, and a test in an Android module runs
under `./gradlew testDebugUnitTest`.

## 2. The two Gradle test tasks

This is the detail that trips every new team:

| Task | Which modules |
| --- | --- |
| `./gradlew test` | The **pure JVM** modules: `:core:kernel`, `:feature:notes:contracts`, `:feature:notes:domain`, `:evals` |
| `./gradlew testDebugUnitTest` | The **Android** modules: `:core:ui-core`, `:feature:notes:data`, `:feature:notes:ui`, `:app` |

An Android module also has `testReleaseUnitTest`. The root `test` task does
**not** cover the Android modules, because their task is named
`testDebugUnitTest`. A CI job that runs only `./gradlew test` is running
half the suite.

## 3. The layout

The tests live **inside the module they test**, because Gradle requires the
JVM and the instrumented test source sets to be there:

```text
core/kernel/src/test/kotlin/          # the Outcome and the error codes
feature/notes/domain/src/test/kotlin/ # the entity and the use cases
feature/notes/ui/src/test/kotlin/     # the ViewModel
evals/src/test/kotlin/                # the .claude contract tests
```

Four modules have tests. `:feature:notes:contracts` has **none** yet, and
`:feature:notes:data` has none either: the in-memory adapter is a fixed
list of two notes, and a test of a fixed list proves nothing.

The **route parity test** belongs in `:feature:notes:contracts` because the
routes do ([06](06-navigation-routes-and-deep-links.md)) — and because that
module is pure JVM, the test needs no device. That is a direct payoff of
the module split. The test itself is **not implemented**: it lands with the
shared route table ([06](06-navigation-routes-and-deep-links.md) §4).

## 4. Fakes, not mocks

There is no mocking framework in the dependencies. The fakes are
hand-written objects that implement the ports — in the shipped tests, an
anonymous object at the call site:

```kotlin
// The shape of a fake repository: it implements the port the domain
// declared, and it returns exactly the state the test needs to assert on.
val repository = object : NoteRepository {
    override suspend fun list(): Outcome<List<Note>> = Outcome.Success(listOf(note))
}
```

A fake that is ten lines long is cheaper than a framework, and it makes the
port boundary visible: if a fake gets awkward, the port is wrong.

The rule: **a port is never tested against a real implementation.** The use
cases are tested against the fake; the fake is not the thing under test.

| Test | Against |
| --- | --- |
| The use cases | The fake repository |
| The ViewModel | The real use cases, with the fake repository underneath |
| The screen (instrumented) | The real ViewModel |

## 5. The architecture check is a gate

`scripts/check-architecture.sh` reads the build files and the sources and
fails when the module graph is wrong
([03](03-layers-modules-and-udf.md)).

It is a **gate, not a lint**: it exits non-zero, so CI fails. The same
rules are enforced twice on purpose — the compiler catches the impossible
mistakes (an `android.*` import in a pure JVM module), and the script
catches the possible ones (a `ui → data` dependency).

## 6. The evals

`:evals` is a **pure JVM JUnit module** that validates the `.claude` layer:
the agents, the skills and the commands against the schemas in
`.claude/schemas/`, the manifest posture, and the placeholder leak.

```sh
./gradlew :evals:test
```

Two properties of the evals:

| Property | Why |
| --- | --- |
| They are **pure JVM** | The `.claude` layer is text; validating it needs no device |
| The leak detector's tokens are **assembled at runtime**, not written literally | A literal token would be rewritten by `scripts/rename.sh` and the detector would be defused |

The evals are the gate for the kit itself. A change to an agent, a skill or
a command is finished when `:evals:test` is green.

`evals/run-live.sh` is the **live tier**: it drives the `claude` CLI against
a skill and, with `TIER=3`, a judge pass over a canned review. It costs API
budget and needs a live CLI, so it is never in the default gate —
`verify-template.sh` only proves it is a no-op without `EVALS=1`.

## 7. The commands

```sh
bash scripts/check-architecture.sh   # the module graph
./gradlew :app:assembleDebug         # it compiles
./gradlew test                       # the pure JVM unit tests
./gradlew testDebugUnitTest          # the Android unit tests
./gradlew :app:lintDebug             # Android Lint
./gradlew :evals:test                # the .claude contract tests
```

`make ci` runs the first five in one command — a linter the kit ships is a
linter the gate runs.

`bash scripts/verify-template.sh` is the gate above the gate: it stages a
renamed copy of the kit under `/tmp` and runs the whole set on it, so a
break in the substitution contract fails before the release, not after it.

| Command | What it runs |
| --- | --- |
| `./gradlew :app:assembleDebug` | The debug build of the app |
| `./gradlew :app:assembleRelease` | The release build |
| `./gradlew :app:lintDebug` | Android Lint |
| `./gradlew test` | The JVM tests of the pure JVM modules |
| `./gradlew testDebugUnitTest` | The JVM tests of the Android modules |
| `./gradlew :evals:test` | The `.claude` contract tests |
| `./gradlew :feature:notes:domain:test` | The tests of one pure JVM module |
| `./gradlew :feature:notes:ui:testDebugUnitTest` | The tests of one Android module |
| `./gradlew projects` | The module tree |
| `./gradlew tasks --all` | Every task in every module |

## 8. The gate order

The order in the CI job, cheapest first:

| # | Step | Why here |
| --- | --- | --- |
| 1 | `bash scripts/check-architecture.sh` | Milliseconds, and it catches the mistake that wastes the rest |
| 2 | `./gradlew :app:assembleDebug` | The compile proof; it needs the SDK, so it is the slow one |
| 3 | `./gradlew test` | Fast |
| 4 | `./gradlew testDebugUnitTest` | Fast, but it needs the Android SDK |
| 5 | `./gradlew :app:lintDebug` | It needs the SDK and the built variants, so it goes with the build |
| 6 | `./gradlew :evals:test` | Fast |

## 9. Instrumented tests

Instrumented tests need a **device or an emulator**; they are not part of
the fast gate.

| What | Where | When |
| --- | --- | --- |
| The Compose screens, per status | `feature/notes/ui/src/androidTest/` | The screen's behaviour is only observable on a device |
| The Keystore | `core/security/src/androidTest/` | There is no JVM equivalent |
| The notification channels and the `PendingIntent` | `app/src/androidTest/` | The API is Android-only |
| The E2E journeys | A separate module or a CI job | The full flow |

The rules:

| Rule | Why |
| --- | --- |
| An instrumented test is a **JVM test that could not be** | If it can run on the JVM, it runs on the JVM |
| The E2E suite runs on **real devices** from the pool, not only emulators | The Low tier is the target ([02](02-device-tiers-and-the-quality-governor.md)) |
| No production data in a test | The data is the liability |
| A flaky test is quarantined with an owner and an expiry | A flaky test in the gate trains the team to ignore the gate |
| Every route and every notification type has a contract test | The routes are typed ([06](06-navigation-routes-and-deep-links.md)) |

## 10. Writing a test

| Rule | Why |
| --- | --- |
| One behaviour per test; name the behaviour | A failing test names one thing |
| Use the fake, never the real adapter | The test must not need a disk or a network |
| Assert the value, not the log | A test that reads the log breaks when the log changes |
| Use a `TestScope` dispatcher | A test that sleeps is a flaky test |
| Never a real `Dispatchers.IO` in a unit test | The test then depends on the machine's load |
| No `Thread.sleep`, no wall-clock dependency | The same |

## 11. The red test before the code

The loop is the same as in the other kits: write the failing test, watch it
fail **for the right reason** — a missing symbol, not a typo — then write
the smallest change that makes it pass.

```sh
./gradlew :feature:notes:domain:test --tests "*LoadNotesTest*"
```

The `--tests` filter works on any Gradle test task, including the Android
ones.

## Do / don't

| Do | Don't |
| --- | --- |
| Put the test in the module it covers | Collect every test in one folder |
| Run `test` **and** `testDebugUnitTest` | Assume `test` covers the Android modules |
| Test the port against a fake | Test the use case against the real adapter |
| Run the architecture check in CI | Rely on a review to catch the graph |
| Keep the evals green with every `.claude` change | Edit an agent without running `:evals:test` |

## See also

- [Layers, Modules & UDF](03-layers-modules-and-udf.md) — what the
  architecture check enforces.
- [Playbooks](13-playbooks.md) — the recipe for adding a test.
- [Project Layout & Build Logic](05-project-layout-and-build-logic.md) — the
  task names.
