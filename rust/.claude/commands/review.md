---
name: review
description: "Delegate to the review skill — run the <app> pre-landing checklist (unsafe justification, unwrap/expect discipline, SQL audit, clippy) before landing a change."
division: core
---

# /review

## Delegate

Delegate to the **review** skill. Activate when the request is a pre-landing
checklist pass on `<app>` covering unsafe justification, `unwrap`/`expect`
discipline in library paths, `sqlx` query auditing (parameterized binds, schema
drift), and clippy cleanliness
against `just ci`.
