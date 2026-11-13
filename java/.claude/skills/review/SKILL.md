---
name: "review"
description: "Java / Jakarta EE workflow for the <app> template: review."
division: backend
---

# <app> review skill

## When to Activate
Activate when the request is to review a Java/Jakarta EE change: a JAX-RS resource, a use case, a JPA adapter, a migration, a module boundary or a diff. Do not activate for a pure documentation edit or a dependency bump with no behavior change.

## Core Principles
- The use case owns the JTA transaction; the resource is thin.
- Entities never cross the API; DTOs are records with validation.
- Every query carries the tenant predicate; `tenant_id` + RLS are the second line.
- The outbox row commits with the domain row; every async effect is idempotent.
- The error envelope is the platform's; no stack trace or SQL leaks.

## Rules
- Every finding cites file and line, the rule and the smallest fix.
- A broken transaction/tenant contract is CRITICAL or HIGH.
- A `CHECKSTYLE:OFF`/`@SuppressFBWarnings` without a reason is a finding.
- No fabricated gate result: run `mvn verify`, Checkstyle and the two-tenant tests or say you did not.
- The review ends with a gate status and a verdict.

## Research References
| Claim | Source | Year | Link |
|---|---|---|---|
| Jakarta EE 11 is the current platform | Jakarta EE specification | 2026 | https://jakarta.ee/specifications/ |
| MicroProfile Config ordinals | MicroProfile Config spec | 2026 | https://microprofile.io/ |

## Output Format
Severity-ranked findings, then gate status, then APPROVE/WARN/BLOCK. One finding per line: `file:line — rule — fix`.
