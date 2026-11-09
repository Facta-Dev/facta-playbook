// Package apperr carries the typed application error and the stable error
// codes shared by all slices. The HTTP adapter translates an AppError into
// the {"detail": ..., "code": ...} envelope; nothing else touches constants.
package apperr

import "errors"

// AppError is the single error shape crossing slice boundaries.
type AppError struct {
	HTTP   int    // suggested HTTP status
	Code   string // stable machine-readable code (CLIENT_* / AUTH_* / ADMIN_*)
	Detail string // human-readable detail; never leak internals
}

func (e *AppError) Error() string { return e.Code + ": " + e.Detail }

// E builds a new AppError.
func E(httpStatus int, code, detail string) *AppError {
	return &AppError{HTTP: httpStatus, Code: code, Detail: detail}
}

// WithDetail returns a copy with a specific detail, keeping code + status.
func (e *AppError) WithDetail(d string) *AppError {
	return &AppError{HTTP: e.HTTP, Code: e.Code, Detail: d}
}

// From coerces any error into an *AppError (internal fallback).
func From(err error) *AppError {
	var ae *AppError
	if errors.As(err, &ae) {
		return ae
	}
	return E(500, "INTERNAL_ERROR", "internal server error")
}

// Stable codes shared across slices.
const (
	CodeUnauthorized  = 401
	CodeForbidden     = 403
	CodeNotFound      = 404
	CodeConflict      = 409
	CodeUnprocessable = 422
	CodeInternal      = 500

	AuthInvalidCredentials = "AUTH_INVALID_CREDENTIALS"
	AuthAccountInactive    = "AUTH_ACCOUNT_INACTIVE"
	AuthAccountBlocked     = "AUTH_ACCOUNT_BLOCKED"
	AuthEmailNotVerified   = "AUTH_EMAIL_NOT_VERIFIED"
	AuthInvalidToken       = "AUTH_INVALID_TOKEN"
	AuthInvalidRefresh     = "AUTH_INVALID_REFRESH"
	AuthUnauthorized       = "AUTH_UNAUTHORIZED"
	AuthForbidden          = "AUTH_FORBIDDEN"

	Validation             = "VALIDATION_ERROR"
	NotFound               = "NOT_FOUND"
	EmailTaken             = "AUTH_EMAIL_TAKEN"
	EmailTakenAct          = "AUTH_EMAIL_TAKEN_ACTIVE"
	CodeExpired            = "AUTH_CODE_EXPIRED"
	CodeInvalid            = "AUTH_CODE_INVALID"
	CodeUsed               = "AUTH_CODE_USED"
	AuthSignedUpWithSocial = "AUTH_SIGNED_UP_WITH_SOCIAL"
	AuthRateLimit          = "AUTH_RATE_LIMITED"
	PasswordWrong          = "AUTH_PASSWORD_WRONG"

	InternalError = "INTERNAL_ERROR"
)
