---
name: review
description: "Delegate to the review skill — run the <app> pre-landing checklist (unsafe-import justification, error-shadowing discipline, sqlc query audit, golangci-lint) before landing a change."
division: core
---

# /review

## Delegate

Delegate to the **review** skill. Activate when the request is a pre-landing
checklist pass on `<app>` covering unsafe-import justification, error-shadowing
discipline in library paths, `sqlc` query auditing, and golangci-lint
cleanliness against `make ci`.