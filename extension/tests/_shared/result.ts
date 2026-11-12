// <app> — the result helpers.
//
// A Result is a value, not an exception: the helpers narrow it so a test can
// assert on the value or on the error without a cast.

import { type Result } from "../../src/shared-kernel/result";

/** The value of a successful result; fails the test otherwise. */
export function expectOk<T, E>(result: Result<T, E>): T {
  if (!result.ok) {
    throw new Error(`expected a successful result, got ${JSON.stringify(result.error)}`);
  }
  return result.value;
}

/** The error of a failed result; fails the test otherwise. */
export function expectError<T, E>(result: Result<T, E>): E {
  if (result.ok) {
    throw new Error("expected a failed result, got a value");
  }
  return result.error;
}
