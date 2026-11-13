# Telemetry

The kit ships **no telemetry and no logger**. There is no sink, no crash
reporter, no performance marker and no log buffer in the template — only the
seams a product fills in. This chapter describes what a product adopts, and
the rules it must adopt them under, so that adopting them is a decision
rather than an accident.

## 1. The logger a product adds

The logger is created once and passed down from the composition root:

```ts
const logger = createLogger({ capacity: 200 });
logger.log("warn", "push registration refused", { code: result.error.code });
```

| Property    | Value                                         | Why                                                    |
| ----------- | --------------------------------------------- | ------------------------------------------------------ |
| Buffer      | The last 200 entries, in memory               | A restart loses it — see the ceiling below             |
| Redaction   | On write, by key name and by value shape      | A secret never enters the buffer                       |
| Sink        | Optional; the logger calls it for every entry | Where the entries would go in a real deployment        |
| `console.*` | Not used in the app or in the packages        | One output path, so nothing is printed twice or missed |

The levels are `debug`, `info`, `warn` and `error`. There is no global: the
logger instance travels through the composition root into the adapters.

> **Known gap.** The template ships **no logger**. `packages/core` holds
> the `Result`, the device tier, the quality governor and the four ports —
> nothing else. The logger above is an adoption step.

## 2. What is redacted

| Pattern     | Examples                                                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Key name    | `token`, `secret`, `password`, `passwd`, `authorization`, `api_key`, `api-key`, `cookie`, `credential`, `refreshToken`, `accessToken` |
| Value shape | `bearer <value>`, `basic <value>`                                                                                                     |

The redaction is applied when the entry is written, **not** when it is read:
a redacted value is never stored at all.

The redaction removes **secrets**, not personal data. A note body, a precise
coordinate or a customer's name passes through. If the buffer is ever going
to leave the device, personal data needs its own policy.

## 3. What must never be logged

| Never logged                         | Why                             |
| ------------------------------------ | ------------------------------- |
| An access or refresh token           | It is a credential              |
| A note body, a message body          | It is the user's content        |
| A full deep link with its parameters | It may carry an identifier      |
| A precise location                   | It is sensitive personal data   |
| A payment detail                     | It is regulated                 |
| A raw attestation verdict            | It is a security signal         |
| A whole HTTP body                    | It may contain any of the above |

A correlation id is safe: it links entries without identifying the user.

## 4. The ceiling

A logger's buffer lives in the JS thread of a process the OS may kill. Two
consequences, both worth stating in the source:

- **The log is not durable.** After a restart the buffer starts empty.
- **The buffer is not a sink.** A sink is optional, and nothing in the
  template sends data anywhere.

## 5. Crash reporting

Not shipped. When a product adopts it, the rules are:

| Rule                                                    | Why                                          |
| ------------------------------------------------------- | -------------------------------------------- |
| Upload source maps and debug symbols privately          | A public map is a source map for an attacker |
| Attach the device tier, the app version and the variant | A crash on the low tier is a different bug   |
| Attach the last N redacted log entries                  | The context of the crash                     |
| Never attach a request body or a token                  | The uploader is a third party                |
| Respect the user's consent and the store's disclosure   | It is a data transfer                        |

## 6. Performance markers

Not shipped. The markers a mobile product wants, in the order they matter:

| Marker            | What it measures                           | The budget it answers        |
| ----------------- | ------------------------------------------ | ---------------------------- |
| `startup.cold`    | Process launch → first frame committed     | ≤ 3.5 s on the low tier      |
| `startup.warm`    | Resume → first frame                       | ≤ 1.0 s                      |
| `screen.tti`      | Navigation → the screen's data is rendered | ≤ 500 ms on a warm network   |
| `js.stall`        | A JS task longer than 16 ms                | Zero above 100 ms            |
| `frame.p95`       | The rolling p95 frame time                 | The governor's budget        |
| `memory.resident` | The process's resident memory              | ≤ 180 MB for the shell       |
| `network.request` | Latency by endpoint and connection type    | The server's p95             |
| `sync.outbox`     | The depth and the age of the outbox        | Zero pending over 15 minutes |

`js.stall` and `frame.p95` are the two that turn a vague "the app is slow"
into a number. The governor consumes the same signal
([02](02-device-tiers-and-the-quality-governor.md)).

## 7. The diagnostics bundle

A user-initiated, redacted bundle is a product decision. When a product
ships one, it contains:

- the app version, the variant and the build
- the device tier and the capabilities
- the quality profile history (the governor's decisions)
- the permission states and the push registration state
- the recent redacted log entries
- the recent errors with their correlation ids

It never contains a token, a note body or a precise location.

## 8. The consent and the disclosure

| Question                        | Where the answer belongs                           |
| ------------------------------- | -------------------------------------------------- |
| Does anything leave the device? | The store listing's privacy section                |
| What is the retention period?   | The sink's configuration                           |
| Is there a consent screen?      | The product, before the first sink is installed    |
| What is sampled?                | The sink; sampling is the cheapest privacy control |

The template's answer to the first question is **no**: nothing leaves the
device. Installing a sink changes that answer, and the privacy disclosure
must change with it.

## 9. What the template ships

| Piece                         | Status                                            |
| ----------------------------- | ------------------------------------------------- |
| The redacting logger          | **Not shipped** — an adoption step, see §1        |
| The redaction rules           | **Not shipped** — they come with the logger       |
| The correlation id convention | **Not shipped** — a product decision              |
| A crash reporter              | **Not shipped** — a vendor and a privacy decision |
| Performance markers           | **Not shipped** — a product decision              |
| A remote sink                 | **Not shipped** — see §8                          |
| A diagnostics bundle          | **Not shipped** — a product decision              |

## Do / Don't

| Do                                         | Don't                                     |
| ------------------------------------------ | ----------------------------------------- |
| Pass one logger down from the root         | Create a logger per module                |
| Redact by key name and by value shape      | Assume the payload is safe to log         |
| Attach the tier and the version to a crash | Attach the whole log buffer verbatim      |
| State the retention of a sink              | Add a sink that ships data off by default |
| Use `logger.log` instead of `console.*`    | Mix `console.*` and the logger            |

## See also

- [Security & Secure Store](09-security-and-secure-store.md)
- [Device Tiers & the Quality Governor](02-device-tiers-and-the-quality-governor.md)
- [Testing & CI](12-testing-and-ci.md)
