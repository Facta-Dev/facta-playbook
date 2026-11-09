// Features authsession owns session flows: login (gates + retry lock),
// refresh (token rotation), logout, me/update-me, and the session-management
// views (own + admin + anomalies). The access token is a short-lived JWT
// never stored; one durable sessions row per refresh token. Port of the Go
// template's authsession slice.

import {
  E,
  CodeUnauthorized,
  CodeForbidden,
  CodeNotFound,
  CodeUnprocessable,
  AuthInvalidCredentials,
  AuthInvalidRefresh,
  AuthAccountInactive,
  AuthAccountBlocked,
  AuthEmailNotVerified,
  NotFound,
  AuthSignedUpWithSocial,
  Validation,
} from "#shared/apperr.js";

// UserRow is this slice's view of the users table. The nullable profile
// columns read as null (pg returns null); `null` = leave-unchanged in
// update-me inputs.
export interface UserRow {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  passwordHash: string;
  role: string;
  userType: string;
  language: string;
  phone: string | null;
  picture: string | null;
  address: string | null;
  postal: string | null;
  subscription: string | null; // profile surface only; not me-updatable
  isActive: boolean;
  isBlocked: boolean;
  isVerified: boolean;
  retryCount: number;
  lastLogin: Date | null;
  createdAt: Date;
  updatedAt: Date;
  googleLinked: boolean; // google_id is set (SignedUpWithSocial parity)
  githubLinked: boolean; // github_id is set
}

// SessionRow is one durable refresh session. createdAt/lastActivity carry
// the audit timestamps surfaced by the session-management views; the
// rotation lookup does not select them and leaves them null — they are
// coalesced with now() in the wire shapes.
export interface SessionRow {
  id: string;
  userId: string;
  refreshToken: string;
  ipAddress: string;
  userAgent: string;
  expiresAt: Date;
  isActive: boolean;
  createdAt: Date;
  lastActivity: Date;
}

// SessionUser is a session joined with the fields needed to mint a token:
// rotation re-checks active-session lookup joins the account's current
// state, so blocked/deactivated users cannot ride an existing session past
// refresh.
export interface SessionUser extends SessionRow {
  email: string;
  role: string;
  userActive: boolean;
  isBlocked: boolean;
  isVerified: boolean;
}

// Error objects reused by every flow in this slice.
export const errInvalidCredentials = E(
  CodeUnauthorized,
  AuthInvalidCredentials,
  "invalid credentials",
);
export const errInvalidRefresh = E(CodeUnauthorized, AuthInvalidRefresh, "invalid refresh token");
export const errAccountInactive = E(CodeForbidden, AuthAccountInactive, "account is inactive");
export const errAccountBlocked = E(CodeForbidden, AuthAccountBlocked, "account is blocked");
export const errEmailNotVerified = E(CodeForbidden, AuthEmailNotVerified, "email not verified");
export const errUserNotFound = E(CodeNotFound, NotFound, "user not found");
// A wrong password tried on an account whose identity came from Google/GitHub:
// the guidance is to sign in with that provider or reset the password.
export const errSignedUpWithSocial = E(CodeForbidden, AuthSignedUpWithSocial, "");
export const errValidationSession = E(CodeUnprocessable, Validation, "window out of range");

// Anomaly is one row of the admin session-anomaly report (reference
// auth_session_detect_anomalies semantics).
export interface Anomaly {
  type: string; // expired_but_active | multi_ip_sessions
  count: number; // affected sessions (expired) or users (multi_ip)
  details: Record<string, unknown> | null;
}

// MultiIPRow is one GROUP-BY result of the anomaly aggregate: a user whose
// active sessions within the window span more distinct IPs than the SQL
// threshold.
export interface MultiIPRow {
  userId: string;
  ipCount: number;
}

// UpdateMeInput carries the optional profile fields; undefined = leave
// unchanged, "" sets the empty string (never null). Mirrors UpdateMeRequest
// with geo fields (country/city) dropped — no countries/cities tables.
export interface UpdateMeInput {
  firstName?: string;
  lastName?: string;
  phone?: string;
  picture?: string;
  language?: string;
  address?: string;
  postal?: string;
}
