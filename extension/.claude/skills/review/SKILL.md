---
name: "<app> Review"
description: "Runs the <app> extension pre-landing checklist — layer boundaries, sender validation, protocol coverage, storage versioning, permission posture and test adequacy — producing a severity-ranked verdict against `make ci`."
division: core
---

# <app> Review

A reusable pre-landing review checklist for the `<app>` Chrome extension. An agent invokes this skill; the agent supplies the contextual judgement, the skill supplies the deterministic procedure. Six fixed categories are walked in the same order every time: layer boundaries, sender validation, protocol coverage, storage versioning, permission posture, and test adequacy. The output is raw structured prose with a verdict, never a fenced block.

## When to Activate

- Before landing any change under `src/`, `entrypoints/`, `tests/` or the manifest.
- Whenever a request asks for a "review", a "gate check" or a "pre-landing pass".
- A pull request touches a handler, an adapter, the protocol, the storage store or the manifest.
- A follow-up review after fixes from a prior `/review` pass.

Do not activate for documentation-only changes with no source edits. This skill COMPLEMENTS the `code-reviewer` core agent rather than replacing it: the agent reasons freely about intent and tradeoffs, while this skill walks the same six categories in the same order every time.

## Core Principles

- **Checklist, not judgement.** Six fixed categories, always in this order: layer boundaries, sender validation, protocol coverage, storage versioning, permission posture, test adequacy.
- **`chrome.*` belongs to `src/platform/`.** Any use of the global outside that folder is a BLOCK; ESLint and dependency-cruiser enforce it, and the review states which rule caught it.
- **Validate, then parse.** A handler that parses a payload before the sender's id, origin, frame and class are checked is a BLOCK.
- **Contracts are shared.** Every message type needs a schema, a `PROTOCOL` entry, a client call, a server handler, a contract test and a sender-class test. A type missing the last two is a BLOCK.
- **Durable data is versioned.** A changed record shape without a version bump, a pure migration and a fixture of the previous version is a BLOCK.
- **Permissions are justified.** A new grant without a `docs/permissions-register.md` entry is a BLOCK; `host_permissions` must stay empty and `debugger` must be absent.

## Rules

- No fabricated data: do not invent file paths or line numbers. If a file was not read this session, list it under "not reviewed" instead of guessing.
- Every claim needs a source: a finding cites the file and line actually read, or it is tagged "unverified".
- No code-fenced final output where raw output is expected: the verdict is raw structured prose.
- Read each touched file before reporting on it; unread files are listed separately.
- BLOCK only on a real finding: a layer violation, a parse-before-validate handler, a message type without its two tests, an unbumped storage version, a new permission without a register entry, a blanket host grant, or a `debugger` permission.
- WARN on style-grade issues: a thin but present justification, a duplicated constant that belongs in the contracts, a test that could assert more precisely.
- APPROVE only when all six categories show no BLOCK-level finding.

## Research References

Provenance table — every claim in a finding needs a row.

| claim | source | year | link |
| --- | --- | --- | --- |
| The `chrome` global is restricted outside `src/platform/**` | `eslint.config.mjs` | 2024 | local file |
| The architecture rules `no-chrome-outside-platform`, `no-cross-feature-imports`, `no-circular` and the rest | `dependency-cruiser.config.cjs` | 2024 | local file |
| The sender checks and their order | `src/security/sender.ts` | 2024 | local file |
| The envelope, the payload schemas and the `PROTOCOL` allowlist | `src/contracts/protocol.ts` | 2024 | local file |
| The current storage version and the migration chain | `src/storage/notes-store.ts` | 2024 | local file |
| The manifest posture: storage, sidePanel, alarms, no host permissions | `src/manifest.ts` | 2024 | local file |
| The test tiers and the gates | `PLAYBOOK_TESTS.md` | 2024 | local file |
| The extension messaging model and the sender trust boundary | Chrome extension messaging documentation | 2024 | https://developer.chrome.com/docs/extensions/develop/concepts/messaging |

## Output Format

Raw structured prose — not a code fence:

```
### <app> review — pre-landing checklist

Gate: make ci (typecheck + lint + test + build)

Findings:
- [BLOCK|WARN] layer-boundary @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] sender-validation @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] protocol-coverage @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] storage-versioning @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] permission-posture @ <file>:<line> — <what> | fix: <suggestion>
- [BLOCK|WARN] test-adequacy @ <file>:<line> — <what> | fix: <suggestion>

Not reviewed (not read):
- <file>

Verdict: APPROVE | WARN | BLOCK
```
