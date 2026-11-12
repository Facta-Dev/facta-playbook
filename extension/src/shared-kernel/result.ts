// <app> — the Result type.
//
// Errors cross a layer boundary as values, never as exceptions
// (PLAYBOOK §6.1): a failed message is answered with a typed error, so a
// handler never throws inside the service worker.

export type Result<T, E = unknown> =
  { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: E };

/** A successful result. */
export function ok<T, E = never>(value: T): Result<T, E> {
  return { ok: true, value };
}

/** A failed result. */
export function err<T = never, E = unknown>(error: E): Result<T, E> {
  return { ok: false, error };
}
