// Features adminusers owns the admin user-management endpoints: paginated
// list, single fetch and partial update behind requireRole(Admin), with an
// audit_logs trail written on every mutation.

import {
  E,
  CodeForbidden,
  CodeNotFound,
  CodeUnprocessable,
  AuthForbidden,
  NotFound,
  Validation,
} from "#shared/apperr.js";

// Row is the admin view of one user.
export interface Row {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: string;
  userType: string;
  isActive: boolean;
  isBlocked: boolean;
  isVerified: boolean;
  language: string;
  createdAt: Date;
  lastLogin: Date;
}

// Patch is the nullable update applied to a user.
export interface Patch {
  isActive: boolean | null;
  isBlocked: boolean | null;
  role: string; // "" leaves the role untouched
}

// UpgradeResult is the outcome of the unauthenticated whitelist upgrade.
export interface UpgradeResult {
  email: string;
  message: string;
  upgraded: boolean; // false reads as "the account already was an admin"
}

// userType ADMIN; slice-local because cross-slice imports are forbidden —
// authsignup owns the other user_type constants for provisioning.
export const userTypeAdmin = "ADMIN";

// Slice errors.
export const errNotFound = E(CodeNotFound, NotFound, "user not found");
export const errBadRole = E(CodeUnprocessable, Validation, "unknown role");
export const errNoChanges = E(CodeUnprocessable, Validation, "empty update");
// errNotWhitelisted: the bootstrap upgrade denies any email absent from
// <APP>_ALLOWED_ADMIN_EMAILS before a user row is even looked up.
export const errNotWhitelisted = E(
  CodeForbidden,
  AuthForbidden,
  "email is not authorized to upgrade to admin",
);
