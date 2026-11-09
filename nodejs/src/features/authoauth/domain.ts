// Features authoauth owns the social logins: Google One-Tap (a verified ID
// token provisions-or-links the account, then issues the same login shape
// as the email flow) and GitHub OAuth (authorize redirect + callback that
// ALWAYS answers 302 — tokens ride the URL fragment on success and
// ?error=/error_description= on failure, never JSON).

import {
  E,
  CodeUnauthorized,
  CodeForbidden,
  CodeNotFound,
  CodeUnprocessable,
  AuthInvalidToken,
  AuthInvalidCredentials,
  AuthForbidden,
  AuthAccountInactive,
  AuthAccountBlocked,
  NotFound,
  Validation,
} from "#shared/apperr.js";

// User is the minimal user projection this slice needs.
export interface User {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  password: string; // only on provisioning: unusable random secret, never read back
  picture: string | null;
  role: string;
  userType: string;
  language: string;
  googleId: string; // "" = not linked (NULL reads as "")
  githubId: string;
  isActive: boolean;
  isBlocked: boolean;
  isVerified: boolean;
}

// SessionRow is one durable refresh session (mirrors the sessions table).
export interface SessionRow {
  id: string;
  userId: string;
  refreshToken: string;
  ipAddress: string;
  userAgent: string;
  expiresAt: Date;
  isActive: boolean;
}

// Result is the login-shape payload (same fields the email login returns).
export interface Result {
  accessToken: string;
  refreshToken: string;
  expiresAt: Date;
}

// LinkStatePrefix marks a callback state as account-linking ("link:<jwt>");
// anything else is a plain login.
export const LinkStatePrefix = "link:";

// Frontend redirect labels (reference parity).
export const errGithubNotConfig = "github_not_configured";
export const errAuthFailed = "auth_failed";
export const errNoCode = "no_code";
export const linkSuccessRedirect = "/user/profile?tab=overview&github_linked=1";
export const authSuccessSuffix = "/auth/success";

// Errors of this slice. Google sign-in answers 401 like bad credentials;
// a disabled GitHub provider is an operator problem and answers 403 like
// the admin surface (openapi: both github routes document the 403 Envelope).
// Inactive/blocked match the email login's statuses (authsession parity).
export const errInvalidGoogleToken = E(CodeUnauthorized, AuthInvalidToken, "invalid google token");
export const errGoogleEmailLinked = E(
  CodeUnauthorized,
  AuthInvalidToken,
  "email is linked to another account",
);
export const errGithubAlreadyLinked = E(
  CodeUnauthorized,
  AuthInvalidToken,
  "github account is already linked to another user",
);
export const errGoogleNotConfigured = E(
  CodeUnauthorized,
  AuthInvalidCredentials,
  "google sign-in is not configured",
);
export const errGithubNotConfigured = E(
  CodeForbidden,
  AuthForbidden,
  "github sign-in is not configured",
);
export const errInvalidLinkState = E(CodeUnauthorized, AuthInvalidToken, "invalid or expired link");
export const errAccountInactive = E(CodeForbidden, AuthAccountInactive, "account is inactive");
export const errAccountBlocked = E(CodeForbidden, AuthAccountBlocked, "account is blocked");
export const errUserNotFound = E(CodeNotFound, NotFound, "user not found");
export const errValidation = E(CodeUnprocessable, Validation, "invalid request");
