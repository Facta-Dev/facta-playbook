# Security & CSP

A Chrome extension runs privileged code inside somebody else's browser. The
kit's posture is: **deny by default, validate at every boundary, and refuse
to load remote code at all.**

## 1. The boundaries and the checks at each one

| Boundary                  | What crosses it              | The check                                                              |
| ------------------------- | ---------------------------- | ---------------------------------------------------------------------- |
| Web page → content script | DOM events, page data        | The content script holds no secret and trusts no page value            |
| Content script → worker   | A runtime message            | `classifySender`: the id, the url, the frame and the manifest patterns |
| Any message → handler     | The envelope and the payload | `MessageHeaderSchema`, then `RequestSchema`, then the response schema  |
| Worker → storage          | A record                     | `NotesRecordSchema` (strict) plus the domain rules                     |
| Administrator → extension | The managed policy           | `ManagedPolicySchema`; invalid values are ignored                      |
| Extension → network       | Nothing                      | The kit ships no outbound call                                         |

## 2. The default CSP

Manifest V3 forbids remotely hosted code. The extension runs under the
default policy

```text
script-src 'self'; object-src 'self'
```

The manifest of this kit declares **no** `content_security_policy` key, so
the default applies. The consequences:

- No `eval`, no `new Function`, no remote `<script>`.
- No inline event handlers (`onclick="..."`) in the HTML; React attaches the
  handlers.
- Only scripts packaged with the extension may run.

The Vite build inlines the React runtime into the bundle and emits no inline
script, so the default policy is sufficient. If a project needs a stricter
policy, it declares `content_security_policy` in `src/manifest.ts` — and only
after reading [the MV3 CSP rules](https://developer.chrome.com/docs/extensions/reference/manifest/content-security-policy).

## 3. What the code refuses

| Refused                                        | Where                                                                                                           |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `chrome` outside `src/platform/**`             | ESLint `no-restricted-globals` + dependency-cruiser `no-chrome-outside-platform` + the static architecture test |
| A feature importing another feature            | ESLint `no-restricted-imports` + dependency-cruiser `no-cross-feature-imports`                                  |
| A message from an unknown sender               | `classifySender` → `forbidden-sender`                                                                           |
| A message from a class the type does not allow | The `PROTOCOL` table → `forbidden-sender`                                                                       |
| An unexpected key in a payload                 | Strict zod objects                                                                                              |
| A record that cannot be parsed                 | The store drops it and backs it up                                                                              |
| A secret in the logs                           | The logger redacts on write                                                                                     |

## 4. The logger redacts

`src/telemetry/logger.ts` masks any key whose name looks like a secret
(`token`, `secret`, `password`, `passwd`, `authorization`, `api_key`,
`api-key`, `cookie`, `credential`) and any value that looks like a bearer or
basic credential. The redaction happens on write, so a redacted value never
reaches the ring buffer.

## 5. Secrets, storage and the page

- **No secret is committed.** The kit ships `.env.example` and no secret
  value; `scripts/leak-guard.sh` fails the tree on a high-signal secret
  pattern.
- **The content script is in an isolated world.** It cannot read the
  extension's storage, and the page cannot read the content script's
  variables.
- **Nothing is exposed to the page.** The manifest declares no
  `web_accessible_resources`, so no extension file is reachable from a web
  page.
- **No `externally_connectable`.** A web page cannot message the extension.

## 6. Known gaps of the template

| Gap                                                     | Consequence                                                                                     | Fix                                                                   |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `managed-schema.json` is not referenced by the manifest | The administrator policy never reaches the extension                                            | Wire `storage.managed_schema` (see the [Admin Guide](admin-guide.md)) |
| The manifest declares no `key`                          | The extension id changes with the install path, so the id in `SenderPolicy` changes per install | Accept it for an unpacked build; set a `key` for a distributed build  |
| The content script pattern is `https://example.com/*`   | It matches a real site                                                                          | Replace it with your own pattern or drop the content script           |

## 7. Worked example

The sender check is a single function with no browser dependency, so it is
unit-tested directly (`tests/_platform/security/sender-allowlist.test.ts`):

```ts
if (sender.id !== policy.extensionId) return err("forbidden-sender");
if (url === undefined) return err("forbidden-sender");
if (sender.frameId !== undefined && sender.frameId !== 0) return err("forbidden-sender");
if (url.startsWith(`${policy.extensionOrigin}/`)) return ok("ui");
```

## Do / Don't

| Do                                                    | Don't                                     |
| ----------------------------------------------------- | ----------------------------------------- |
| Validate the sender before the payload                | Trust a message because the type is known |
| Declare a strict CSP when you need one                | Add `unsafe-eval` to make a build work    |
| Redact at the logger                                  | Redact at every call site                 |
| Keep host permissions empty until a feature needs one | Add `<all_urls>` for convenience          |

## See also

- [Permissions Register](permissions-register.md)
- [Messaging & Sender Validation](07-messaging-and-sender-validation.md)
- [Telemetry](11-telemetry.md)
