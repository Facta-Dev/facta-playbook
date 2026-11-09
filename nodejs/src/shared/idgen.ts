// Shared idgen generates identifiers: UUIDs for rows and opaque, unguessable
// tokens for session refresh.

import { randomBytes, randomUUID } from "node:crypto";

export interface Generator {
  newId(): string;
}

export const uuidGen: Generator = {
  newId: () => randomUUID(),
};

// newToken returns a 64-char URL-safe opaque token (48 random bytes),
// suitable for session refresh tokens.
export const newToken = (): string => randomBytes(48).toString("base64url");

// newCode returns an n-digit numeric code as a string. Rejection-sampling
// keeps every digit uniform in 0..9 (a plain modulo over randomBytes would
// skew the top digits by ~2%).
export function newCode(n: number): string {
  const digits = "0123456789";
  let out = "";
  while (out.length < n) {
    for (const b of randomBytes(n)) {
      if (b < 250 && out.length < n) out += digits[b % 10];
    }
  }
  return out;
}
