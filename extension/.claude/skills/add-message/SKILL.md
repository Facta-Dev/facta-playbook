---
name: "<app> Add Message"
description: "Add a message type to the <app> extension protocol — the shared zod envelope in src/contracts/protocol.ts, the PROTOCOL sender-class entry, the client call, the server handler and the contract plus sender-class tests — as one gated change."
division: client
---

# <app> Add Message

A reusable workflow for adding one message type to the `<app>` extension's protocol. The type is a contract, not a function call: both ends parse the same zod schema, the `PROTOCOL` table says which sender classes may invoke it, and the sender is validated before the payload is parsed. This skill produces all of those pieces together, plus the two tests that every type owes — a contract test and a sender-class test — so a type never lands half-wired.

## When to Activate

- A new message type is needed between the service worker and an extension page.
- A new message type is needed between the service worker and a content script.
- An existing type's payload schema changed in a way that is not backward compatible.
- A reviewer asks whether a proposed type is reachable by the right senders.

Do not activate for a purely local, in-process call inside one slice — that is a function call, not a message, and it needs no protocol entry.

## Core Principles

- One schema, two ends. The envelope `{ v, id, type, correlationId, payload }` and the payload schema live once in `src/contracts/protocol.ts`; the client in `src/messaging/client.ts` and the server in `src/messaging/server.ts` both parse it.
- The `PROTOCOL` table is the allowlist. Each type names the sender classes that may invoke it — `ui` for the extension's own pages, `content` for a page script. A type with no entry is not callable.
- Validate, then parse. The sender id, origin, frame and class are checked before the payload is parsed; a parse-first handler is a BLOCK.
- Errors are values. A failure returns `Result<T, RpcError>` with a stable code; nothing throws across the message boundary, because an unhandled rejection kills the service worker.
- Every reply correlates. The reply carries the `correlationId` of its request, and every request is settled — a handler that never replies stalls the caller.

## Rules

- Anti-hallucination: the type name, the schema and the handler path must be written down in the sketch; never invent a message type that no code references.
- Every claim needs a source: cite the local protocol file for the envelope shape and the official Chrome documentation for the messaging behaviour.
- Do not emit the final output as a fenced code block when raw output is expected.
- A new type ships with both tests: a contract test (both ends parse the same envelope) and a sender-class test (each allowed class works, each disallowed class is rejected). Missing either is a REVISE.
- Do not bump `PROTOCOL_VERSION` for an additive type; bump it only when the envelope shape changes incompatibly, and say which.
- A type that only the content script needs must not be reachable from a `ui` sender by accident, and vice versa — the narrowest class set that works is the correct one.
- No `chrome.*` in the handler: the handler lives behind a port, and the platform owns the transport.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| `chrome.runtime.sendMessage` and the `onMessage` listener are the transport between extension contexts | Chrome extension messaging documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/messaging |
| A message response is asynchronous and must be returned `true` from the listener to keep the channel open | Chrome extension messaging documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/messaging |
| A content script can talk to the extension, but it shares the page's trust boundary | Chrome extension content scripts documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/content-scripts |
| The envelope, the payload schemas and the `PROTOCOL` allowlist live in one file | `src/contracts/protocol.ts` | 2024 | local file |
| The sender checks and their order | `src/security/sender.ts` | 2024 | local file |
| The error codes and the `Result` type | `src/shared-kernel/errors.ts`, `src/shared-kernel/result.ts` | 2024 | local file |

## Output Format

A protocol change sketch plus a test checklist. Plain text — not a fenced block:

```
TYPE  notes:rename

  contract:
    payload schema   src/contracts/protocol.ts — the rename body schema
    envelope         { v, id, type, correlationId, payload }
    PROTOCOL entry   sender classes: [ui]
    protocol version  unchanged (additive)

  client:
    src/messaging/notes.ts — typed call returning Result<T, RpcError>

  server:
    src/messaging/server.ts — handler; validateSender() BEFORE parsing;
    the reply carries the request correlationId

  tests:
    tests/notes/frontend/contract/message-server.test.ts
      - the envelope round-trips through the client and the server
      - an unknown protocol version returns a typed error
    tests/_platform/security/sender-allowlist.test.ts
      - a ui sender is accepted for notes:rename
      - a content sender is rejected for notes:rename

CHECKLIST
  [ ] payload schema added next to the existing ones
  [ ] PROTOCOL entry added with the sender classes the feature needs
  [ ] sender validated before the payload is parsed
  [ ] handler returns a Result; nothing throws across the boundary
  [ ] contract test + sender-class test written
  [ ] pnpm run typecheck + pnpm run test green
```

Summary line: `add-message: N type(s) added, M sender class(es) — verdict: APPROVE | REVISE`.
