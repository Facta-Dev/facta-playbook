---
name: tdd-cycle
description: Run one Red-Green-Refactor slice: write the failing test first, make it pass with minimum code, then refactor. Use for each task in tasks.md.
when_to_use: "run a tdd cycle", "/tdd-cycle [task]", "implement T-00X test-first", "red green refactor"
argument-hint: [T-00X]
---

# One TDD cycle

## Procedure

1. Read `specs/<slug>/tasks.md` and pick the next open task (or the one named).
   If none is open, stop.
2. Detect the test runner, in this order, and stop with a clear error if none
   is found:
   - `scripts/tdd/run-tests <slice>`
   - `scripts/tdd/slice-test <slice>`
   - `go.mod` → `go test ./...`
   - `Cargo.toml` → `cargo test`
   - `pyproject.toml` → `pytest`
   - `package.json` → its `test` script
3. **RED** — write only the test(s) for this task's acceptance criteria. Run
   the runner and confirm they fail for the expected reason. Record the
   command and the failing output.
4. **GREEN** — write the minimum production code that passes. Run the runner
   until green. Do not change the test to make it pass.
5. **REFACTOR** — clean up with the suite green; re-run after each change.
6. Mark the task `[x]` and note the run evidence (command + result) next to it.
7. Set the spec `status: in-progress` on the first cycle if it is `planned`.

## Rules

- Never write production code before a recorded failing test.
- One slice per invocation; do not batch unrelated tasks.
- If a test fails for the wrong reason, fix the test setup, not the assertion.
- If the task cannot be done as specified, stop and report — do not silently
  shrink scope.
