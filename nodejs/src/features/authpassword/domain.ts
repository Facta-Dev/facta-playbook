// Features authpassword owns the password flows over email codes: request a
// reset code (rate-limited), validate a code, reset (auto-verifies, optional
// revoke-all), and change (re-verified current password; other sessions
// revoked, current session kept via jti == sessions.id).

import {
  E,
  CodeUnauthorized,
  CodeUnprocessable,
  AuthRateLimit,
  CodeExpired,
  CodeInvalid,
  CodeUsed,
  PasswordWrong,
  Validation,
} from "#shared/apperr.js";

// Reset is a pending password-reset code row.
export interface Reset {
  id: string;
  userId: string;
  email: string;
  token: string;
  expiresAt: Date;
  used: boolean;
  attempts: number; // wrong-code guesses; capped by maxCodeAttempts
  createdAt: Date; // set by the store (DB default); used for rate limiting
}

// maxCodeAttempts caps wrong-code guesses per code: with a 6-digit space and
// a 30m window, validate/reset must not be an unthrottled oracle.
export const maxCodeAttempts = 5;

// User is the minimal user projection the flows need.
export interface User {
  id: string;
  email: string;
  firstName: string; // "" in the by-ID lookup (zero-value parity)
  password: string; // from the by-ID lookup (change flow); "" in by-email
  language: string;
}

// Errors of this slice. Expired/invalid/used are 400s; rate limit is 429.
export const errRateLimited = E(429, AuthRateLimit, "too many reset requests");
export const errCodeExpired = E(400, CodeExpired, "reset code expired");
export const errCodeInvalid = E(400, CodeInvalid, "reset code invalid");

export const errCodeUsed = E(400, CodeUsed, "reset code already used");
export const errPasswordWrong = E(CodeUnauthorized, PasswordWrong, "current password is wrong");
export const errValidation = E(CodeUnprocessable, Validation, "invalid request");
