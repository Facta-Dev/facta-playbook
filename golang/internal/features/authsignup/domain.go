// Package authsignup owns signup: create an unverified user, email the
// 6-digit verification code, verify it, and re-send it. Idempotent contract:
// re-signup of an existing unverified email re-issues the code (201); a
// verified email is a 409 conflict.
package authsignup

import (
	"strings"
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// UserRow is this slice's view of the users table (slices don't share models).
type UserRow struct {
	ID           string
	Email        string
	FirstName    string
	LastName     string
	PasswordHash string
	Role         string
	UserType     string
	Language     string
	IsActive     bool
	IsVerified   bool
}

// The user types accepted at signup (reference parity): the request value is
// optional and defaults to a company account. The users.user_type CHECK
// (migrations 0001) constrains the same set.
const (
	UserTypeAdmin   = "ADMIN" // no profile table
	UserTypeCompany = "COMPANY"
	UserTypeUser    = "USER"
)

// NormalizeUserType resolves the optional request value to the stored
// user_type: blank falls back to the COMPANY default; anything else is
// accepted case-insensitively only if it names a known type.
func NormalizeUserType(t string) (string, bool) {
	t = strings.ToUpper(strings.TrimSpace(t))
	switch t {
	case "":
		return UserTypeCompany, true
	case UserTypeAdmin, UserTypeCompany, UserTypeUser:
		return t, true
	default:
		return "", false
	}
}

// Verification is a pending email code.
type Verification struct {
	ID        string
	UserID    string
	Token     string
	ExpiresAt time.Time
	Used      bool
	Attempts  int // wrong-code guesses; capped by maxCodeAttempts
}

// maxCodeAttempts caps wrong-code guesses per code: with a 6-digit space and
// a 24h window, verification must not be an unthrottled oracle.
const maxCodeAttempts = 5

// Error constants reused by every flow in this slice.
var (
	ErrEmailTakenActive = apperr.E(apperr.CodeConflict, apperr.EmailTakenAct, "email already registered")
	ErrEmailTaken       = apperr.E(apperr.CodeConflict, apperr.EmailTaken, "email already in use") // lost insert race
	ErrCodeExpired      = apperr.E(400, apperr.CodeExpired, "verification code expired")
	ErrCodeInvalid      = apperr.E(400, apperr.CodeInvalid, "verification code invalid")
	ErrCodeUsed         = apperr.E(400, apperr.CodeUsed, "verification code already used")
	ErrValidation       = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request")
)
