# Telemetry

Three different things share one name, and the template separates them:

| Kind | Where it goes | Who reads it | In the template |
| --- | --- | --- | --- |
| **Diagnostics** | A local buffer on the device | The user, support | **Not implemented** — the logger is specified here, not shipped |
| **Analytics** | A remote endpoint | The product team | **Not implemented** |
| **Crash reports** | A remote endpoint | The product team | **Not implemented** |
| **Performance markers** | A remote endpoint, or a local trace | The product team | **Not implemented** |

> **Known gap — do not copy a fiction.** The template ships **no logger, no
> ring buffer, no diagnostics screen and no sink**. The eight modules do not
> contain a telemetry one. This chapter is the **specification** a project
> adopts, plus the adoption path in §9.

The template refuses all four: each of them needs a consent or a product
decision a template cannot make for you.

## 1. The logger

The logger writes to **two places at once**: the platform log (`android.util.Log`, filterable with `adb logcat`) and an **in-memory ring buffer** of the last N entries, oldest first. The ring buffer is what a diagnostics bundle reads.

| Piece | What it is |
| --- | --- |
| The category | A closed enum — `App`, `Storage`, `Network`, `Navigation`, `Push` — not a free-text string |
| The message | Short and greppable |
| The fields | The identifiers you need to correlate, and nothing else |
| The redaction | **On by default**, applied before anything is stored or printed |

A new category is a new enum value, not a string invented at a call site.

## 2. Redaction

The logger redacts **before** anything is stored or printed.

| Mechanism | What it catches |
| --- | --- |
| `redactText` | A bearer token in free text: the pattern becomes `Bearer ***` |
| `isSensitiveKey` | A field whose name contains `token`, `access_token`, `refresh_token`, `password`, `secret`, `authorization`, `api_key` or `cookie` — its value becomes `***` |

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
| The access or refresh token | The `correlationId` |
| The password | The error code |
| The reset token from a deep link | The route name |
| The OTP | The outcome |
| The precise location | The city or the coarse tile |
| The note body | The note count, or a truncated title only with an explicit product decision |
| The full API response | The status code and the error code |
| The Keystore key material | Nothing — it is not exportable |
| The email address | A hashed user id |

## 4. Crash reporting

Not implemented in the template. When a project adds it, the constraints
are not optional:

| Rule | Why |
| --- | --- |
| The crash reporter is **consent-aware** | A crash report contains the app state at the moment of the crash |
| The **mapping file** is uploaded for the release build | Without it, the crash report is a stack of obfuscated names |
| The native symbols are uploaded too | An NDK crash without symbols is unreadable |
| The report is tagged with the variant, the tier and the device class | A crash on a Low device is a different bug from a crash on a High one |
| The report is **redacted** before it leaves the device | A crash report carries the whole memory of the app, including the session |
| The previous exit reason is ingested | `ApplicationExitInfo` tells you about the memory-limiter kills that never produced a crash report ([01](01-platform-model-and-android-runtime.md)) |

The device class tag is the one that is easy to forget and the one that
changes what you do next: a crash rate that is 0.2 % overall and 4 % on the
Low pool is a performance problem, not a code problem.

## 5. Performance markers

A **marker** is a named span with a duration, recorded on the frame the work
started.

| Marker | Measures |
| --- | --- |
| `startup` | Cold start, from the process start to the first frame |
| `screen.<name>` | The time to the first meaningful frame of a screen |
| `network.<endpoint>` | The duration of a call, per endpoint and network type |
| `db.<query>` | The duration of a query |
| `sync.drain` | The duration of an outbox drain and the number of mutations |

The rules:

| Rule | Why |
| --- | --- |
| A marker is a **named constant**, not a free-text string | A typo silently creates a second metric |
| The markers are recorded on the frame the work started | A marker measured after the work is a lie |
| The work happens off the main thread | A marker on the main thread measures the main thread |
| `reportFullyDrawn()` is called when the screen is complete | It is what the platform measures for the TTFD budget ([02](02-device-tiers-and-the-quality-governor.md)) |
| The tracing is sampled on the Low tier | Tracing is not free on a 2-core device |

The platform's own signals complement the markers: `JankStats` for the
frames, `ApplicationExitInfo` for the exits, and the **Play vitals** in the
Play Console for the aggregate crash, ANR and startup rates per device
model.

## 6. The diagnostics bundle

The bundle is what a user attaches to a support ticket. The template provides
none of the ingredients; every row below is a thing a project adds.

| Item | From |
| --- | --- |
| The ring buffer entries | The logger |
| The app version and the version code | `BuildConfig` |
| The device model, the OS version and the tier | The device module ([02](02-device-tiers-and-the-quality-governor.md)) |
| The effective policy | The managed policy, redacted ([Admin Guide](admin-guide.md)) |
| The push status and the channel states | The push module ([10](10-push-notifications.md)) |
| The recent errors, with their correlation ids | The logger |

It must **not** contain the token, the keystore material, the note bodies,
the database file, or anything outside the app's own directories.

## 7. The three rules of logging

| Do | Don't |
| --- | --- |
| Log the **outcome** and the error code | Log the payload |
| Use the category | Invent a string per call site |
| Log the identifier you need to correlate | Log the object dump |
| Log at the layer that **decides** | Log the same failure in three layers |
| Redact by default | Log first, redact later |

**Log where you decide, not where you notice.** Three layers logging the
same failure is the most common defect in a client app: the storage layer
logs the SQL error, the adapter logs the storage error, and the ViewModel
logs the adapter error. Three lines, one failure, no way to tell which one
mattered.

## 8. The template today

| Piece | State |
| --- | --- |
| The redacting logger | **Not implemented** — specified in §1, adopted per §9 |
| The in-memory ring buffer | **Not implemented** |
| The diagnostics bundle builder | **Not implemented** |
| A remote analytics sink | **Not implemented** — and it needs a consent flow |
| A crash reporter | **Not implemented** |
| The performance markers | **Not implemented** |
| A consent flow | **Not implemented** — nothing is collected, so none is needed |

## 9 · The adoption path

1. **Add `:core:logging`** — a pure JVM module: the category enum, the
   entry type, the redaction, and the ring buffer. Pure JVM so the
   redaction is unit-testable without a device; the `android.util.Log`
   call sits behind an interface the `:app` module implements.
2. **Add the redaction test** — a `redactText` and `isSensitiveKey` test
   with a token, a password and an email address in the input. Redaction
   that is not tested is decoration.
3. **Add the `LogSink` implementation** in `:app` — the platform log plus
   the ring buffer, behind the injected interface.
4. **Add the diagnostics screen** in `:app` — it reads the ring buffer and
   the effective configuration, and it is gated by the `DiagnosticsEnabled`
   managed key ([Admin Guide](admin-guide.md) §2).
5. **Add the bundle builder** — the assembled file, with the exclusions of
   §6 enforced.
6. **Only then**, add a remote sink — with the consent flow and the
   constraints of §10.

Nothing leaves the device until step 6. The order is the safety property:
the redaction is in place before there is anything to redact, and the
consent flow is in place before there is anything to consent to.

## 10. When you add a remote sink

The constraints are not optional:

1. **Consent first**, and the default is off. The managed policy can turn it
   off for an organisation ([Admin Guide](admin-guide.md)).
2. **The same redaction** applies to the payload, plus a **whitelist** of
   the fields that may leave the device. A whitelist, not a blacklist: a
   field that nobody thought about must not ship by default.
3. **A separate queue on disk**, with a size cap, so a network outage does
   not grow an unbounded file.
4. **The endpoint is named in the admin guide**, and the policy key that
   disables it is documented there too.
5. **No request at all** when the policy forbids it — not a request with a
   flag attached.

## Do / don't

| Do | Don't |
| --- | --- |
| Log the outcome and the correlation id | Log the response body |
| Redact before writing | Log first, redact later |
| Log once, where the decision is made | Log the same failure in three layers |
| Tag a crash with the variant and the device class | Ship an untagged crash report |
| Use a whitelist for what leaves the device | Ship every field by default |

## See also

- [Security & Keystore](09-security-and-keystore.md) — the secrets the
  redactor protects.
- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
  — the budgets the performance markers measure.
- [Admin Guide](admin-guide.md) — the policy key that turns telemetry off.
- [Playbook Native Android Kotlin §16](../PLAYBOOK_MOBILE_ANDROID_KOTLIN.md)
  — the observability signals.
