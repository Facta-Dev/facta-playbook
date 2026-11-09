---
name: spec-verify
description: Prove traceability from acceptance criteria to tests to passing results, and produce verify.md. Use when a spec's tasks are done, before merging.
when_to_use: "verify the spec", "/spec-verify", "is specs/<slug> done", "acceptance evidence"
argument-hint: <slug>
---

# Verify a spec

## Procedure

1. Read `spec.md` and `tasks.md`. List every AC- and the tests that claim it.
2. Run the test suite using the same runner detection as `tdd-cycle`
   (`scripts/tdd/run-tests` → `scripts/tdd/slice-test` → `go.mod` /
   `Cargo.toml` / `pyproject.toml` / `package.json`; stop if none).
3. Build the traceability table: AC → test file → last result.
4. Write `specs/<slug>/verify.md` with the table, the commands run, the date,
   and any AC that lacks a test or a green result.
5. Report gaps plainly. Do not claim coverage you did not observe.
6. Propose `status: verified` — **change the status only after the user
   confirms**. Then the spec may move to `closed` after merge.

## Verification gate

- Every AC maps to a test and a passing result.
- Slice tests are green; review findings are closed.
- Any exception is written down with the reason.

## Never

- Do not weaken or delete a test to reach green.
- Do not edit production code during verification; report and stop.
