# Telemetry

Three different things share one name, and the template separates them:

| Kind | Where it goes | Who reads it | In the template |
| --- | --- | --- | --- |
| **Diagnostics** | A local buffer on the device | The user, support | **Not implemented** — the logger is specified here, not shipped |
| **Analytics** | A remote endpoint | The product team | **Not implemented** |
| **Crash reports** | A remote endpoint | The product team | **Not implemented** |
| **Performance markers** | A remote endpoint, or a local trace | The product team | **Not implemented** |

> **Known gap — do not copy a fiction.** The template ships **no logger, no
> ring buffer, no diagnostics screen and no sink**. The six packages do not
> contain a telemetry one, and no source file in the tree calls `print` or
> `Logger`. This chapter is the **specification** a project adopts, plus the
> adoption path in §9.

The template refuses all four: each of them needs a consent or a product
decision a template cannot make for you.

## 1. The logger

The logger writes to **two places at once**: the unified system log
(`os.Logger`, readable from Console.app and from `log stream`) and an
**in-memory ring buffer** of the last N entries, oldest first. The ring
buffer is what a diagnostics bundle reads.

| Piece | What it is |
| --- | --- |
| The category | A closed enum — `app`, `storage`, `network`, `navigation`, `push` — not a free-text string |
| The subsystem | The bundle id |
| The message | Short and greppable |
| The fields | The identifiers you need to correlate, and nothing else |
| The redaction | **On by default**, applied before anything is stored or printed |

A new category is a new enum value, not a string invented at a call site.

The OSLog privacy annotation is the second line of defence, and it is a
compiler-checked one:

```swift
// The token is interpolated into the log call, and the annotation says the
// value is private: the log shows a placeholder, and the real value is
// only visible to a process entitled to read it.
private let logger = Logger(subsystem: "com.example.app", category: "network")
logger.info("POST \(path) → \(status), token=\(token, privacy: .private)")
```

`privacy: .private` is the mechanism the platform gives; the redaction rule
below is the policy that says what must never be logged in the first place.
The annotation redacts a value someone chose to log. It does not make
logging a token acceptable.

## 2. Redaction

The logger redacts **before** anything is stored or printed.

| Mechanism | What it catches |
| --- | --- |
| `redactText` | A bearer token in free text: the pattern becomes `Bearer ***` |
| `isSensitiveKey` | A field whose name contains `token`, `access_token`, `refresh_token`, `password`, `secret`, `authorization`, `api_key` or `cookie` — its value becomes `***` |
| `privacy: .private` | A value the call site marks private, at the point of interpolation |

Redaction is **on by default** and can only be turned off deliberately. That
deserves a review comment.

The rule behind it: **the logger is not a place to put a secret, and it
redacts anyway.** A log line with a token is a defect even when the
redaction saves it — the token was in memory, in the heap, and possibly in a
screenshot of the log.

What is **not** redacted, and therefore must never be logged:

| Never logged | Why |
| --- | --- |
| A note's title or body | It is the user's content |
| A precise location | It is sensitive personal data |
| An email address, a phone number, a name | PII |
| A refresh token, in any form | The redactor is a safety net, not a control |
| A file path outside the app's directories | It may name a user directory |
| The full request or response body | It contains everything above |

## 3. What must never be logged

The list, as a checklist:

| Never | Use instead |
| --- | --- |
| The access or refresh token | The `correlationID` |
| The password | The error code |
| The reset token from a deep link | The route name |
| The OTP | The outcome |
| The precise location | The city or the coarse tile |
| The note title or body | The note count, or a truncated title only with an explicit product decision |
| The full API response | The status code and the error code |
| The Keychain key material | Nothing — it is not exportable |
| The email address | A hashed user id |

The `Note` entity in this template is an id and a title, and **the title is
the user's content**. A logger that prints a `Note` prints the user's text.
The domain type stays out of the log call; the count and the status go in.

## 4. Correlation ids

The correlation id is the thread that ties a user's report to the server's
record. The template already has one: `AppError` carries an optional
`correlationID`, and `UiStatus.error(correlationID:)` shows it on screen.

| Rule | Why |
| --- | --- |
| The id is generated **per user action** | One action, one id, from the tap to the server |
| It travels with the request, in a header | The server's log and the client's log share the id |
| It comes back in the error envelope | The client shows the id the server used |
| It is safe to log | An id identifies an action, not a person |

`AppError(code:correlationID:)` is the whole mechanism today: the use case
converts a thrown error into `Outcome.failure`, the model maps the code to a
`UiStatus`, and the screen renders the id when there is one. A logger that
records the id makes the whole chain reconstructable from the device.

## 5. Crash reporting

Not implemented in the template. When a project adds it, the constraints
are not optional:

| Rule | Why |
| --- | --- |
| The crash reporter is **consent-aware** | A crash report contains the app state at the moment of the crash |
| The **dSYMs** are uploaded for the release build | Without them, the crash report is a stack of unresolved addresses |
| The report is tagged with the variant, the tier and the device class | A crash on a Low device is a different bug from a crash on a High one |
| The report is **redacted** before it leaves the device | A crash report carries the whole memory of the app, including the session |
| The previous-launch termination reason is ingested | MetricKit's payloads carry the jetsam and the memory-pressure kills that never produced a crash report |

### The options

| Option | What it gives | The note |
| --- | --- | --- |
| **MetricKit** | The system's own diagnostics: `MXCrashDiagnostic`, `MXHangDiagnostic`, `MXCPUExceptionDiagnostic`, plus the launch, hang-rate, memory and disk payloads | **First-party, no dependency.** The trade-off is that the payloads arrive on a delay and are not queryable in the moment |
| **Crashlytics** (Firebase) | Crash and hang reports with symbolicated stacks, out of the box | A third-party SDK: a dependency-register entry, a privacy manifest, and a network dependency |
| **Sentry Cocoa** | Crashes, hangs and performance, with self-hosting | The same obligations as Crashlytics, plus the operational ones |

The template ships **zero third-party packages**, and every one of these is
one. The playbook's decision D20 lists **MetricKit + os.Logger/signposts +
Sentry Cocoa (or Crashlytics)**, and MetricKit is the only one of the
three that adds no dependency; a project that needs crash reports with
symbolication in the same minute chooses the SDK and accepts the register
row and the privacy manifest that come with it.

The device-class tag is the one that is easy to forget and the one that
changes what you do next: a crash rate that is 0.2 % overall and 4 % on the
Low pool is a performance problem, not a code problem.

## 6. Performance markers

A **marker** is a named span with a duration, recorded on the frame the work
started.

| Marker | Measures |
| --- | --- |
| `startup` | Cold start, from the process start to the first frame |
| `screen.<name>` | The time to the first meaningful frame of a screen |
| `network.<endpoint>` | The duration of a call, per endpoint and network type |
| `store.<query>` | The duration of a query |
| `sync.drain` | The duration of an outbox drain and the number of mutations |

The rules:

| Rule | Why |
| --- | --- |
| A marker is a **named constant**, not a free-text string | A typo silently creates a second metric |
| The markers are recorded on the frame the work started | A marker measured after the work is a lie |
| The work happens off the main actor | A marker on the main actor measures the main actor |
| `os_signpost` intervals, not string parsing | The signpost is a structured interval the Instruments tools read |
| The tracing is sampled on the Low tier | Tracing is not free on a 2-core device |
| The governor's decisions are recorded | A tier step-down that nobody sees is a bug nobody fixes ([02](02-device-tiers-and-the-quality-governor.md)) |

The platform's own signals complement the markers: the **MetricKit** payloads
for the aggregate launch, hang and memory rates, and the Xcode Organizer
for the field trends per device model.

## 7. The diagnostics bundle

The bundle is what a user attaches to a support ticket. The template
provides none of the ingredients; every row below is a thing a project adds.

| Item | From |
| --- | --- |
| The ring buffer entries | The logger |
| The app version and the build number | `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION` in `project.yml` |
| The device model, the OS version and the tier | The device module ([02](02-device-tiers-and-the-quality-governor.md)) |
| The effective policy | The managed configuration, redacted ([Admin Guide](admin-guide.md) §5) |
| The push status and the category states | The push module ([10](10-push-notifications.md)) |
| The recent errors, with their correlation ids | The logger |

It must **not** contain the token, the Keychain material, the note bodies,
the store file, or anything outside the app's own directories.

The bundle is produced **on the user's request** and gated by the
`DiagnosticsEnabled` managed key. An administrator who cannot see what the
app believes cannot debug a deployment
([Admin Guide](admin-guide.md) §10).

## 8. The three rules of logging

| Do | Don't |
| --- | --- |
| Log the **outcome** and the error code | Log the payload |
| Use the category | Invent a string per call site |
| Log the identifier you need to correlate | Log the object dump |
| Log at the layer that **decides** | Log the same failure in three layers |
| Redact by default | Log first, redact later |

**Log where you decide, not where you notice.** Three layers logging the
same failure is the most common defect in a client app: the store logs the
write error, the adapter logs the store error, and the model logs the
adapter error. Three lines, one failure, no way to tell which one mattered.

## 9. The template today

| Piece | State |
| --- | --- |
| The redacting logger | **Not implemented** — specified in §1, adopted per §10 |
| The in-memory ring buffer | **Not implemented** |
| The diagnostics bundle builder | **Not implemented** |
| A remote analytics sink | **Not implemented** — and it needs a consent flow |
| A crash reporter | **Not implemented** — MetricKit is the dependency-free option |
| The performance markers | **Not implemented** |
| A consent flow | **Not implemented** — nothing is collected, so none is needed |

## 10. The adoption path

1. **Add `Packages/Core/Logging`** — a package with the category enum, the
   entry type, the redaction, and the ring buffer. A pure package, so the
   redaction is unit-testable with `swift test` and no device; the `Logger`
   call sits behind an interface the app target implements.
2. **Add the redaction test** — a `redactText` and `isSensitiveKey` test
   with a token, a password and an email address in the input. Redaction
   that is not tested is decoration.
3. **Add the `LogSink` implementation** in the app target — the `Logger`
   plus the ring buffer, behind the injected interface.
4. **Add the diagnostics screen** in the app target — it reads the ring
   buffer and the effective configuration, and it is gated by the
   `DiagnosticsEnabled` managed key ([Admin Guide](admin-guide.md) §5).
5. **Add the bundle builder** — the assembled file, with the exclusions of
   §7 enforced.
6. **Only then**, add a remote sink — with the consent flow and the
   constraints of §11.

Nothing leaves the device until step 6. The order is the safety property:
the redaction is in place before there is anything to redact, and the
consent flow is in place before there is anything to consent to.

## 11. When you add a remote sink

The constraints are not optional:

1. **Consent first**, and the default is off. The managed configuration can
   turn it off for an organisation, through the `TelemetryLevel` key
   (`off`, `errors`, `full`) ([Admin Guide](admin-guide.md) §5).
2. **The same redaction** applies to the payload, plus a **whitelist** of
   the fields that may leave the device. A whitelist, not a blacklist: a
   field that nobody thought about must not ship by default.
3. **A separate queue on disk**, with a size cap, so a network outage does
   not grow an unbounded file.
4. **The endpoint is named in the admin guide**, and the policy key that
   disables it is documented there too.
5. **No request at all** when the policy forbids it — not a request with a
   flag attached.
6. **The remote sink is a dependency**: a register row, a privacy manifest,
   and a review note, in the same change.

## Do / don't

| Do | Don't |
| --- | --- |
| Log the outcome and the correlation id | Log the response body |
| Redact before writing | Log first, redact later |
| Mark a private value with `privacy: .private` | Rely on the annotation to excuse a token |
| Log once, where the decision is made | Log the same failure in three layers |
| Tag a crash with the variant and the device class | Ship an untagged crash report |
| Use a whitelist for what leaves the device | Ship every field by default |
| Prefer MetricKit when no SDK is needed | Add a crash SDK without a register row |

## See also

- [Security & Keychain](09-security-and-keychain.md) — the secrets the
  redactor protects.
- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
  — the budgets the performance markers measure.
- [Admin Guide](admin-guide.md) — the policy key that turns telemetry off.
- [Dependency Register](dependency-register.md) — the row a crash-reporting
  SDK needs.
- [Playbook Native iOS Swift §16](../PLAYBOOK_MOBILE_IOS_SWIFT.md) — the
  observability signals.
