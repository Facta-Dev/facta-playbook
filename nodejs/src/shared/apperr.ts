// Shared apperr carries the typed application error and the stable error
// codes shared by all slices. The HTTP adapter translates an AppError into
// the {"detail": ..., "code": ...} envelope; nothing else touches constants.

// AppError is the single error shape crossing slice boundaries.
export class AppError extends Error {
  readonly http: number; // suggested HTTP status
  readonly code: string; // stable machine-readable code (AUTH_* / VALIDATION_*)
  readonly detail: string; // human-readable detail; never leak internals

  constructor(http: number, code: string, detail: string) {
    super(`${code}: ${detail}`);
    this.name = "AppError";
    this.http = http;
    this.code = code;
    this.detail = detail;
  }
}

// E builds a new AppError.
export const E = (httpStatus: number, code: string, detail: string): AppError =>
  new AppError(httpStatus, code, detail);

// withDetail returns a copy with a specific detail, keeping code + status.
export const withDetail = (e: AppError, d: string): AppError => E(e.http, e.code, d);

// from coerces any thrown value into an AppError (internal fallback).
export const from = (err: unknown): AppError =>
  err instanceof AppError ? err : E(CodeInternal, InternalError, "internal server error");

// Stable codes shared across slices.
export const CodeUnauthorized = 401;
export const CodeForbidden = 403;
export const CodeNotFound = 404;
export const CodeConflict = 409;
export const CodeUnprocessable = 422;
export const CodeInternal = 500;

export const AuthInvalidCredentials = "AUTH_INVALID_CREDENTIALS";
export const AuthAccountInactive = "AUTH_ACCOUNT_INACTIVE";
export const AuthAccountBlocked = "AUTH_ACCOUNT_BLOCKED";
export const AuthEmailNotVerified = "AUTH_EMAIL_NOT_VERIFIED";
export const AuthInvalidToken = "AUTH_INVALID_TOKEN";
export const AuthInvalidRefresh = "AUTH_INVALID_REFRESH";
export const AuthUnauthorized = "AUTH_UNAUTHORIZED";
export const AuthForbidden = "AUTH_FORBIDDEN";

export const Validation = "VALIDATION_ERROR";
export const NotFound = "NOT_FOUND";
export const EmailTaken = "AUTH_EMAIL_TAKEN";
export const EmailTakenAct = "AUTH_EMAIL_TAKEN_ACTIVE";
export const CodeExpired = "AUTH_CODE_EXPIRED";
export const CodeInvalid = "AUTH_CODE_INVALID";
export const CodeUsed = "AUTH_CODE_USED";
export const AuthSignedUpWithSocial = "AUTH_SIGNED_UP_WITH_SOCIAL";
export const AuthRateLimit = "AUTH_RATE_LIMITED";
export const PasswordWrong = "AUTH_PASSWORD_WRONG";

export const InternalError = "INTERNAL_ERROR";
