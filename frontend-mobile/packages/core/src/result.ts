// <app> — Result: errors are values, never exceptions thrown across a layer.
//
// A mobile boundary fails in ordinary ways (no network, no permission, a
// rejected promise from a native module). Those failures are part of the
// contract, so they travel as data.

/** A successful outcome. */
export interface Ok<T> {
  readonly ok: true;
  readonly value: T;
}

/** A failed outcome. */
export interface Err<E> {
  readonly ok: false;
  readonly error: E;
}

/** Either a value or an error — never both, never neither. */
export type Result<T, E> = Ok<T> | Err<E>;

/** Wraps a value in a successful outcome. */
export function ok<T>(value: T): Ok<T> {
  return { ok: true, value };
}

/** Wraps an error in a failed outcome. */
export function err<E>(error: E): Err<E> {
  return { ok: false, error };
}

/** Narrows a result to its successful half. */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
  return result.ok;
}

/** Narrows a result to its failed half. */
export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
  return !result.ok;
}
