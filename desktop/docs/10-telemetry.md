# 10 — Telemetry

## What "telemetry" means here

Three different things share one name, and the template separates them:

| Kind | Where it goes | Who reads it | In the template |
| --- | --- | --- | --- |
| **Diagnostics** | A local file, on the user's disk | The user, support | **Shipped** — the ring buffer and the Qt logging category |
| **Analytics** | A remote endpoint | The product team | **Not implemented** |
| **Crash reports** | A remote endpoint | The product team | **Not implemented** |

The template ships the first: a desktop app without a local log is undebuggable.
It refuses the other two: they need a consent decision a template cannot make.

## The local logger

`desktop::infrastructure::telemetry::Logger` writes to two places at once:
the Qt logging category `desktop.telemetry` (info level, through `qCInfo`,
filterable with `QT_LOGGING_RULES`), and an **in-memory ring buffer** of the
last **512** entries (the default `ringCapacity`), oldest first. The ring
buffer is what the diagnostics bundle reads.

```cpp
enum class Category { App, Http, Storage, Realtime, Platform };

void log(Category category, const QString& message, QMap<QString, QString> fields = {});
```

A new category is a new enum value, not a free-text string.

## Redaction

The logger redacts **before** anything is stored or printed. Two mechanisms:

| Mechanism | What it catches |
| --- | --- |
| `redactText` | A bearer token in free text: the pattern `(?i)\bbearer\s+[A-Za-z0-9._~+/=-]+` becomes `Bearer ***` |
| `isSensitiveKey` | A field whose name contains `token`, `access_token`, `refresh_token`, `password`, `secret`, `authorization`, `api_key` or `cookie` — its value becomes `***` |

Redaction is **on by default** and can be turned off only deliberately
(`setRedactionEnabled(false)`), a decision worth a review comment.

The rule behind it: **the logger is not a place to put a secret, and it
redacts anyway.** A log line with a token is a defect even if the redaction
saves it: the token is in memory, in a crash dump and in a screenshot.

What is **not** redacted and must therefore never be logged: a note's title
or body, a file path outside the app's directories, a personal name, an email
address. The list is a safety net for credentials, not a privacy filter.

## The diagnostics bundle

The bundle is what a user attaches to a support ticket. The template
provides the ingredients; assembling the file is a gap.

| Item | From |
| --- | --- |
| The ring buffer entries | `Logger::entries()` |
| The application version | `QApplication::applicationVersion()`, i.e. `DESKTOP_VERSION` |
| The platform and Qt versions | `QSysInfo` |
| The schema version | `NoteStore::schemaVersion()` |
| The effective policy | The managed policy, redacted ([Admin Guide](admin-guide.md)) |
| The integrity check | `PRAGMA integrity_check` |
| The log files | The log directory from `AppPaths` |

It must **not** contain the token, the keystore file, the note bodies, the
database file itself, or anything under the user's home outside the app's
own folders.

> **Known gap.** No bundle builder exists in the template. The seam is
> `Logger::entries()` plus `AppPaths::logDir()`; the file format, the
> redaction pass and the "attach to a ticket" UX are product work.

## Logging rules

| Do | Don't |
| --- | --- |
| Log the **outcome** and the error code | Log the payload |
| Use the category | Invent a new string per call site |
| Log the identifier you need to correlate | Log the object dump |
| Keep a message short and greppable | Write a sentence that changes every release |
| Log at the layer where the decision is made | Log the same failure in three layers |

**Log where you decide, not where you notice.** Three layers logging the same
failure is the most common defect in desktop apps: storage logs the SQL error,
the adapter logs the storage error, and the view model logs the adapter error.

## What the template does not do

| Not implemented | Consequence |
| --- | --- |
| No remote analytics sink | The product team has no usage data; the local log is the only source |
| No crash reporter | A crash is visible only if the user collects the log |
| No metrics pipeline | No latency, error-rate or usage counters |
| No consent flow | Nothing is collected, so none is needed — until an analytics sink is added |
| No crash-loop detection | The safe-mode hook of [03](03-no-gpu-fallback.md) is not wired |
| The logger is not wired into `main.cpp` | The component is implemented and unit-tested, but the composition root does not construct a `Logger` |

The last row is the honest gap: the logger is a component, not a dependency of
the app yet. Wiring it is one line in `main.cpp` plus a file sink.

## When you add a remote sink

If a product decision adds an analytics endpoint, the constraints are not
optional:

1. **Consent first**, and the default is off. The policy key
   `telemetry_level` exists for exactly this.
2. **The same redaction** applies to the payload, plus a whitelist of the
   fields that may leave the device.
3. **A separate queue** on disk, with a size cap, so a network outage does
   not grow an unbounded file.
4. **The endpoint is named in the admin guide**, and an enterprise can turn
   it off through the managed policy ([Admin Guide](admin-guide.md)).
5. **No request at all** when the policy forbids it — not a request with a
   flag.

## Testing the logger

`src/infrastructure/tests/tst_runtime.cpp` covers the logger. Two tests
matter more than the rest: a message containing a bearer token comes back
redacted, and a field named `access_token` comes back as `***`. They guard
the one place in the app where a mistake leaks a credential.

## Do / don't

| Do | Don't |
| --- | --- |
| Redact by default | Log first, redact later |
| Log the error code and the identifier | Log the payload or the body |
| Keep the ring buffer small and in memory | Write an unbounded log file |
| Treat the local log as the user's data | Ship it to a server without consent |

## See also

- [Playbook Desktop Qt §13](../PLAYBOOK_DESKTOP_QT.md) — the telemetry rules.
- [Admin Guide](admin-guide.md) — the `telemetry_level` policy key.
- [Storage & Migrations](08-storage-and-migrations.md) — the log and data
  directories.
