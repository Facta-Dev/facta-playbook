# Glossary

Every term used in this set, defined once. Where a term has an Android
meaning and a kit meaning, the kit meaning is the one that counts here.

**Adapter** — An implementation of a port that talks to the outside world.
`InMemoryNoteRepository` in `:feature:notes:data` implements
`NoteRepository`.

**AGP** — The Android Gradle Plugin. The build plugin that turns a module
into an Android app or library. The template pins 9.5.1 in the version
catalog.

**Android vitals** — The platform's aggregate crash, ANR, jank and startup
metrics, reported per device model in the Play Console. The release gate is
a vitals threshold, not a unit test.

**App Links** — Verified deep links: `https://<domain>/…` with
`android:autoVerify="true"` and an `assetlinks.json`. A custom scheme is
the weaker fallback.

**Application** — The Android component that runs once per process, before
any Activity. The template ships **no** `Application` subclass; the
composition root is the `App()` composable in `:app`.

**Baseline profile** — A list of classes and methods that should be
pre-compiled into the ART image, so the app starts faster. Generated on
real devices and shipped with the release.

**Build type** — `debug` or `release`. The release build is where R8,
shrinking and the signing configuration live.

**Channel (notification)** — An API 26+ container that groups notifications
and carries their importance. Created at app start; its importance cannot
be changed after creation — a change needs a new channel id.

**Compose** — The Jetpack UI toolkit. The template uses it with Material 3
and the Compose BOM 2026.09.00.

**Contracts** — The published face of a slice: the `NavKey` route types and
the interfaces other slices may call. A **pure JVM** module.

**Correlation id** — An identifier that ties a user action, the client's
logs and the server's logs together. It is the one identifier that is safe
to log ([11](11-telemetry.md)).

**Cross-slice access** — Reaching into another slice. Allowed **only**
through that slice's `contracts` module.

**Data (quadrant)** — The `data` module of a slice: the adapters that
implement the domain's ports. An Android library.

**Deep link** — An incoming link from outside the app. **Untrusted**, always
validated against the route table.

**Device tier** — `low`, `mid` or `high`. A classification of the device's
capabilities that selects a quality profile ([02](02-device-tiers-and-the-quality-governor.md)).

**Domain** — The rules of a feature: entities, invariants and the use cases
that apply them. Pure Kotlin, JVM-only, no Android, no Compose.

**Domain (quadrant)** — The `domain` module of a slice. A **pure JVM**
module.

**Effect** — A one-shot action that leaves the screen: navigate, show a
snackbar, request a permission. Delivered through a `Channel`, never as
part of the state.

**Emulator** — A software device on the developer's machine. Useful, but it
is **not** a low-end device: the performance budgets are measured on real
hardware.

**Entry provider** — The Navigation3 lambda that maps a `NavKey` type to a
screen. It lives in `:app`, and it is the only place a route is paired with
a composable.

**Event** — What the user did, travelling **up** from the screen to the
ViewModel. `NotesEvent` in this kit.

**Evals** — The JUnit contract tests in `:evals` that validate the
`.claude` layer: the agents, the skills, the commands, the manifest posture
and the placeholder leaks.

**Expiration** — A pending mutation that is too old to be sent. The outbox
expires it rather than retrying forever.

**Force-stop / process death** — The system killing the process. Everything
that must survive lives in storage, not in a field.

**Foreground service** — A service with a visible notification that runs
while the app is in the background. It requires a **type** and a matching
permission, and Play reviews it.

**Gradle module** — A Gradle project with its own build file and its own
dependency list. The unit of architecture in this kit.

**Hysteresis** — Stepping **up** a quality profile only after a sustained
healthy period, so the app does not oscillate.

**Idempotency key** — A client-generated key that travels with a mutation,
so a retry returns the original result instead of creating a duplicate.

**In-memory adapter** — The repository implementation the template ships: a
fixed list of two notes in a class. It loses everything when the process
dies.

**Instrumented test** — A test that runs on a device or an emulator, in
`src/androidTest`. It can touch the real Android APIs.

**JVM module** — A `kotlin("jvm")` module. **The Android SDK is not on its
classpath**, so an `import android.*` does not compile. This is what makes
the layering a build fact.

**Jank** — A frame that missed its deadline. The user sees a stutter.

**Keystore** — The Android system service that holds a key the app can use
but cannot read. StrongBox is the hardware-backed tier.

**Main thread** — The thread that owns the `Looper` and draws the frame.
Nothing may block on it.

**Module graph** — The set of modules and the dependencies between them.
The architecture, expressed as a build fact.

**NavDisplay** — The Navigation3 composable that renders the back stack.

**NavKey** — A typed navigation route: a `@Serializable` type, in this kit a
sealed interface of routes. A route is a value, not a string.

**Navigation3** — The Jetpack navigation library (1.2.1) whose model is
"the back stack is state you own".

**Notification message / data message** — The two FCM message kinds. A
notification message is user-visible; a data message is a sync hint that
the system may deprioritise.

**Optimistic row** — The local row shown to the user before the server has
confirmed the change.

**Outbox** — The queue of pending mutations. The unit of offline-first
correctness.

**Outcome** — The kernel's sealed result: `Success` or `Failure`. Errors are
values, not exceptions.

**Page size (16 KB)** — The memory page size on recent Android devices. Every
native library in the app must be aligned for it, or Play rejects the
upload.

**PendingIntent** — A token that lets another app start an Activity on the
app's behalf. It must be immutable, and on Android 12+ it must target the
Activity directly.

**Play Integrity** — The platform's verdict on whether the app and the
device look genuine. Evaluated **server-side**; it is a risk signal, not a
client-side block.

**Port** — An interface the consuming module declares, so the use case
depends on the abstraction and not on the implementation. The port belongs
to the **consumer**.

**Pre-launch report** — Play's automated report from the internal test
track. It is the last gate before a staged rollout.

**Projection** — The local copy of server data. Bounded by the server's
data; never the truth.

**Pure JVM module** — See **JVM module**.

**Quality profile** — The value the governor produces: render scale, frame
cap, effects, texture quality, scene tier, particles, prefetch.

**Quota (notification)** — The rate at which a channel may make noise. A
user may lower it; the app may not raise it back.

**Rebased / rejected mutation** — A pending mutation the server refused. It
is surfaced to the user, never dropped silently.

**Route table** — The shared JSON document that defines the URL grammar.
The Kotlin `NavKey` set must match it; the parity test proves it.

**SavedStateHandle** — The ViewModel's handle on the saved state that
survives process death. Small, typed, and validated on the way back in.

**Sealed hierarchy** — A type with a closed set of subtypes. `UiStatus` is
one: one value, not a set of booleans.

**Sensor / capability detection** — Reading what the device can do
(`isLowRamDevice`, the GL ES level, the ARCore availability). A **hint**:
the measured result and the runtime governor win.

**StateFlow** — An observable value holder with a current value. The
ViewModel exposes `StateFlow<NotesState>`.

**StrictMode** — The debug-only watchdog that catches a disk, a network or
a slow call on the main thread. Debug builds only.

**Tink** — Google's crypto library. The AEAD on top of the Keystore key in
this kit.

**Thermal headroom** — The forecast of how close the device is to
throttling. API 30+; it lets the governor step down *before* the throttle.

**Trampoline (notification)** — Routing a notification tap through a
service or a receiver. Forbidden since Android 12.

**UDF** — Unidirectional data flow: state down, events up, effects out.

**UiEvent / UiEffect / UiState** — The three types of the UDF contract
([03](03-layers-modules-and-udf.md)). The shipped slice calls them
`NotesEvent`, `NotesEffect` and `NotesState`.

**Version catalog** — `gradle/libs.versions.toml`: the single source of
every version in the build.

**WorkManager** — The scheduler for deferrable work. The only one that
respects Doze and the standby buckets.

## See also

- [Overview & Principles](00-overview-and-principles.md)
- [Playbooks](13-playbooks.md)
