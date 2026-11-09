// Features authsignup owns signup: create an unverified user, email the
// 6-digit verification code, verify it, and re-send it. Idempotent contract:
// re-signup of an existing unverified email re-issues the code (201); a
// verified email is a 409 conflict. Port of the Go template's authsignup slice.

import {
  E,
  CodeConflict,
  CodeUnprocessable,
  EmailTaken,
  EmailTakenAct,
  CodeExpired,
  CodeInvalid,
  CodeUsed,
  Validation,
} from "#shared/apperr.js";

// UserRow is this slice's view of the users table (slices don't share models).
export interface UserRow {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  passwordHash: string;
  role: string;
  userType: string;
  language: string;
  isActive: boolean;
  isVerified: boolean;
}

// The user types accepted at signup (reference parity): the request value is
// optional and defaults to a company account. The users.user_type CHECK
// constraint (migrations 0001) constrains the same set.
export const UserTypeAdmin = "ADMIN";
export const UserTypeCompany = "COMPANY";
export const UserTypeUser = "USER";

// normalizeUserType resolves the optional request value to the stored
// user_type: blank falls back to the COMPANY default; anything else is
// accepted case-insensitively only if it names a known type (null = invalid).
export const normalizeUserType = (t: string): string | null => {
  const u = t.trim().toUpperCase();
  if (u === "") {
    return UserTypeCompany;
  }
  return [UserTypeAdmin, UserTypeCompany, UserTypeUser].includes(u) ? u : null;
};

// Verification is a pending email code.
export interface Verification {
  id: string;
  userId: string;
  token: string;
  expiresAt: Date;
  used: boolean;
  attempts: number;
}

// maxCodeAttempts caps wrong-code guesses per code: with a 6-digit space and
// a 24h window, verification must not be an unthrottled oracle.
export const maxCodeAttempts = 5;

// Error objects reused by every flow in this slice.
export const errEmailTakenActive = E(CodeConflict, EmailTakenAct, "email already registered");
// The lost insert race maps to the bare AUTH_EMAIL_TAKEN code.
export const errEmailTaken = E(CodeConflict, EmailTaken, "email already in use");
export const errCodeExpired = E(400, CodeExpired, "verification code expired");
export const errCodeInvalid = E(400, CodeInvalid, "verification code invalid");
export const errCodeUsed = E(400, CodeUsed, "verification code already used");
export const errValidation = E(CodeUnprocessable, Validation, "invalid request");
