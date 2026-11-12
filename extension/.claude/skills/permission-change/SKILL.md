---
name: "<app> Permission Change"
description: "Registers a permission or host-pattern change in the <app> extension — the docs/permissions-register.md entry with justification, scope, data touched, owner, review date and alternatives, plus the manifest edit and the test that pins the posture."
division: security
---

# <app> Permission Change

A reusable workflow for adding or changing a capability in the `<app>` extension's manifest. A permission is user-visible consent: it changes the install prompt, the store listing and the attack surface. This skill produces the register entry first, then the manifest edit, then the test that pins the new posture — so the grant is justified and evidenced rather than added as a one-line change.

## When to Activate

- A new entry is added to `permissions` or `optional_permissions`.
- A host pattern is added to `host_permissions`, `optional_host_permissions` or a content script's `matches`.
- A content script's `world`, `run_at`, `all_frames` or `include_globs` changes.
- A reviewer asks whether a proposed grant is justified and whether a narrower alternative exists.

Do not activate for a change that touches no manifest field — a permission-free refactor needs no register entry.

## Core Principles

- The register entry comes first. `docs/permissions-register.md` records, per grant: why it is needed, which feature uses it, the data touched, the owner, the review date and the alternative considered. The manifest edit follows the entry, not the other way round.
- Least privilege is the default. The narrower of the two always wins: an `activeTab` grant over a persistent host permission, an optional permission over a mandatory one, one explicit origin over a domain pattern, and no grant at all over a grant the feature can live without.
- `host_permissions` stays empty. A host is granted per use case, through `optional_host_permissions` or `activeTab`, and each grant is justified individually.
- The `debugger` permission is out of policy — not in `permissions`, not in `optional_permissions`, not proposed in a plan. It is not an alternative this workflow may select.
- The posture is pinned by a test. The native eval asserts the manifest version, the empty `host_permissions`, the absence of `debugger` and of `<all_urls>`, and the content-script match list; a new grant needs its own assertion so a regression fails the gate instead of shipping.

## Rules

- Anti-hallucination: the register entry must name a real feature and a real file; if the feature does not exist yet, say so and stop at the proposal stage.
- Every claim needs a source: cite the local register file and the local manifest source, and the official Chrome documentation for the permission's semantics.
- Do not emit the final output as a fenced code block when raw output is expected.
- Never edit `dist/manifest.json`: it is the build output of `src/manifest.ts`, and the fix goes to the source.
- A blank host pattern such as `<all_urls>` is a BLOCK in either the manifest or a content-script match.
- The `debugger` permission is a BLOCK, always, with no exception path.
- The register entry must name the alternative considered and why it was rejected; an entry that does not is a REVISE.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| Host permissions grant access to origins and are granted at install or on request | Chrome host permissions documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/network-requests |
| `activeTab` grants temporary access to the active tab's origin on a user gesture | `chrome.tabs` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/tabs |
| The `debugger` permission allows inspection and control of the browser tabs | `chrome.debugger` API reference | 2024 | https://developer.chrome.com/docs/extensions/reference/api/debugger |
| The manifest fields, the permissions list and the content-script declaration | Chrome manifest reference | 2024 | https://developer.chrome.com/docs/extensions/reference/manifest |
| The register format and the permission justification rule | `PLAYBOOK_CHROME_EXTENSION.md` | 2024 | local file |
| The posture asserted by the native eval | `evals/contracts.test.mjs` | 2024 | local file |

## Output Format

A register entry, the manifest diff and a test plan. Plain text — not a fenced block:

```
REGISTER ENTRY  docs/permissions-register.md

  permission:  host access to https://example.com
  why:         the notes feature reads the page title the user selected
  feature:     src/features/notes/adapters/in/page-context.ts
  data:        the page title only; never the page body, never a cookie
  owner:       <the team or role that reviews it>
  review on:   <ISO date>
  alternative: activeTab — rejected because the title is read when the
               panel opens, without a user gesture in the tab

MANIFEST  src/manifest.ts (dist/manifest.json is generated — never edited)

  optional_host_permissions: ["https://example.com/*"]
  content_scripts[].matches: ["https://example.com/*"]  (unchanged)

TESTS
  evals/contracts.test.mjs — assert the new optional grant is present and
    that host_permissions stays empty
  tests/_platform/security/sender-allowlist.test.ts — the content sender
    for the granted origin is still class-restricted

CHECKLIST
  [ ] register entry written before the manifest edit
  [ ] alternative considered and the rejection written down
  [ ] no <all_urls> and no debugger
  [ ] grant scoped to the narrowest origin set
  [ ] pnpm run evals + pnpm run test green
```

Summary line: `permission-change: N grant(s) registered, N alternative(s) rejected — verdict: APPROVE | REVISE`.
