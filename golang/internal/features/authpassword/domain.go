// Package authpassword owns the password flows over email codes: request a
// reset code (rate-limited), validate a code, reset (auto-verifies, optional
// revoke-all), and change (re-verified current password; other sessions
// revoked, current session kept via jti == sessions.id).
package authpassword

import (
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// Reset is a pending password-reset code.
type Reset struct {
	ID        string
	UserID    string
	Email     string
	Token     string
	ExpiresAt time.Time
	Used      bool
	Attempts  int       // wrong-code guesses; capped by maxCodeAttempts
	CreatedAt time.Time // set by the store (DB default); used for rate limiting
}

// maxCodeAttempts caps wrong-code guesses per code: with a 6-digit space and
// a 30m window, validate/reset must not be an unthrottled oracle.
const maxCodeAttempts = 5

// User is the minimal user projection the flows need.
type User struct {
	ID       string
	Email    string
	Password string // from the by-ID lookup (change flow)
	Language string
}

// Errors of this slice. Expired/invalid/used are 400s; rate limit is 429.
var (
	ErrRateLimited   = apperr.E(429, apperr.AuthRateLimit, "too many reset requests")
	ErrCodeExpired   = apperr.E(400, apperr.CodeExpired, "reset code expired")
	ErrCodeInvalid   = apperr.E(400, apperr.CodeInvalid, "reset code invalid")
	ErrCodeUsed      = apperr.E(400, apperr.CodeUsed, "reset code already used")
	ErrPasswordWrong = apperr.E(apperr.CodeUnauthorized, apperr.PasswordWrong, "current password is wrong")
	ErrValidation    = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request")
)
