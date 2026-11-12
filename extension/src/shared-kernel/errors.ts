// <app> — the error codes of the extension.
//
// One list, shared by every layer: src/contracts/errors.ts turns it into the
// zod union that both ends of the message protocol validate with, so a code
// that is not in this list can never travel over the wire.

export const ERROR_CODES = [
  "bad-envelope",
  "unknown-type",
  "forbidden-sender",
  "validation-failed",
  "not-found",
  "internal",
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];
