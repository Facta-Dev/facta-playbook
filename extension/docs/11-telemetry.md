# Telemetry

The kit ships **no telemetry to a remote service**. What it ships is a
local, redacting logger: a ring buffer in memory, with a sink you can
replace. That is the whole observability story, and it is deliberately small.

## 1. The logger

`src/telemetry/logger.ts` exposes `createLogger({ capacity = 200, sink })`:

```ts
const logger = createLogger();
logger.log("warn", "message refused", { code: classified.error, senderId: sender.id });
```

| Property    | Value                                         | Why                                                    |
| ----------- | --------------------------------------------- | ------------------------------------------------------ |
| Buffer      | The last 200 entries, in memory               | A worker restart loses it — see the ceiling below      |
| Redaction   | On write, by key name and by value shape      | A secret never enters the buffer                       |
| Sink        | Optional; the logger calls it for every entry | Where the entries would go in a real deployment        |
| `console.*` | Not used anywhere in `src/` or `entrypoints/` | One output path, so nothing is printed twice or missed |

The levels are `debug`, `info`, `warn` and `error`.

## 2. What is redacted

| Pattern     | Examples                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| Key name    | `token`, `secret`, `password`, `passwd`, `authorization`, `api_key`, `api-key`, `cookie`, `credential` |
| Value shape | `bearer <value>`, `basic <value>`                                                                      |

The redaction is applied when the entry is written, not when it is read: a
redacted value is never stored at all.

## 3. The ceiling

The buffer lives in the worker, and the worker dies. Two consequences, both
stated in the source:

- **The log is not durable.** After a restart the buffer starts empty. If
  the log must survive, it must be written to storage (and then it has a
  size and a privacy cost).
- **The buffer is not a sink.** The shipped `sink` is optional and nothing
  in the kit sends data anywhere.

## 4. Where the logger is used

| Call site                     | What it reports                                                                      |
| ----------------------------- | ------------------------------------------------------------------------------------ |
| `src/messaging/server.ts`     | A refused message (`warn`), a failed handler (`error`), an invalid payload (`error`) |
| `src/storage/notes-store.ts`  | A record dropped as corrupt or written by a newer version (`error`)                  |
| `src/policy/reader.ts`        | An invalid administrator policy or invalid user settings (`warn`)                    |
| `entrypoints/options/main.ts` | A policy that could not be read (`error`)                                            |

The same logger instance is passed down through the composition root; there
is no global.

## 5. What a project must add before shipping

The kit is a starting point, not a finished observability story. Before a
real deployment, decide and write down:

| Question                         | Where the answer belongs                        |
| -------------------------------- | ----------------------------------------------- |
| Where do the entries go?         | The `sink` option of `createLogger`             |
| What is the retention period?    | The sink, and the storage policy if it persists |
| Is a session id in the entries?  | Only if it is not personal data                 |
| Does anything leave the machine? | The privacy statement of the store listing      |

Never send a payload, a note body or a URL query to a sink by accident: the
logger redacts _secrets_, not personal data.

## 6. Worked example

A refusal is logged without the content of the message:

```ts
logger.log("warn", "message refused", { code: classified.error, senderId: sender.id });
```

The offending payload is not logged: the point of the entry is the refusal,
and the payload may contain data the extension must not keep.

## Do / Don't

| Do                                      | Don't                                                 |
| --------------------------------------- | ----------------------------------------------------- |
| Pass one logger down from the root      | Create a logger per module                            |
| Redact by key name and by value shape   | Assume the payload is safe to log                     |
| State the retention of a sink           | Add a sink that ships data off the machine by default |
| Use `logger.log` instead of `console.*` | Mix `console.*` and the logger                        |

## See also

- [Service Worker Lifetime](02-service-worker-lifetime.md)
- [Security & CSP](10-security-and-csp.md)
- [Testing & CI](12-testing-and-ci.md)
