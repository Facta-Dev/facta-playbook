---
name: "add-slice"
description: "Java / Jakarta EE workflow for the <app> template: add-slice."
division: backend
---

# <app> add-slice skill

## When to Activate
Activate when the request is to add a new backend capability to the <app> template. Do not activate for a change inside an existing slice's use case (use `plan` + `review`).

## Core Principles
- One slice = one capability, one schema prefix, one URL module, one test folder.
- Domain is plain Java; the use case owns the transaction; the adapter owns JPA.
- The resource is thin; DTOs are records; the error envelope is shared.
- The migration is additive first; `tenant_id` and RLS are in the first migration.

## Rules
- Write the failing use-case test and the ArchUnit rule first.
- Create `features/<slice>/{domain,application,adapters,presentation,contracts}` and register the modules in the reactor.
- Add the Flyway migration with `tenant_id`, the index and the RLS policy.
- Add the OpenAPI entry and the authz matrix row.
- Run `make ci` and `make evals`; no placeholder survives outside the allowed dirs.

## Research References
| Claim | Source | Year | Link |
|---|---|---|---|
| Hexagonal + vertical slice contract | PLAYBOOK_BACKEND.md (J2EE) | 2026 | ../PLAYBOOK_BACKEND.md |
| Flyway + `validate` | Flyway docs | 2026 | https://documentation.red-gate.com/flyway |

## Output Format
An ordered task list (red/green), the files created, the migration name and the gate result.
