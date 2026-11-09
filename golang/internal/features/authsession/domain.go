// Package authsession owns session flows: login (gates + retry lock),
// refresh (token rotation), logout (revoke) and me. The access token is a
// short-lived JWT never stored; one durable sessions row per refresh token.
package authsession

import (
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// UserRow is this slice's view of the users table. The nullable profile
// columns ride as pointers: nil = NULL.
type UserRow struct {
	ID           string
	Email        string
	FirstName    string
	LastName     string
	PasswordHash string
	Role         string
	UserType     string
	Language     string
	Phone        *string // phone_number
	Picture      *string // profile_picture
	Address      *string
	Postal       *string // postal_code
	Subscription *string // profile surface only; not me-updatable
	IsActive     bool
	IsBlocked    bool
	IsVerified   bool
	RetryCount   int
	LastLogin    *time.Time
	CreatedAt    time.Time
	UpdatedAt    time.Time
	GoogleLinked bool // google_id is set (reference SignedUpWithSocial parity)
	GithubLinked bool // github_id is set
}

// SessionRow is one durable refresh session. CreatedAt/LastActivity carry
// the audit timestamps surfaced by the session-management views; the
// rotation lookup does not select them and leaves them zero.
type SessionRow struct {
	ID           string
	UserID       string
	RefreshToken string
	IPAddress    string
	UserAgent    string
	ExpiresAt    time.Time
	IsActive     bool
	CreatedAt    time.Time
	LastActivity time.Time
}

// SessionUser is a session joined with the fields needed to mint a token:
// rotation re-checks account state so blocked/deactivated users cannot ride
// an existing session past Refresh.
type SessionUser struct {
	SessionRow
	Email      string
	Role       string
	UserActive bool
	IsBlocked  bool
	IsVerified bool
}

var (
	ErrInvalidCredentials = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidCredentials, "invalid credentials")
	ErrInvalidRefresh     = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidRefresh, "invalid refresh token")
	ErrAccountInactive    = apperr.E(apperr.CodeForbidden, apperr.AuthAccountInactive, "account is inactive")
	ErrAccountBlocked     = apperr.E(apperr.CodeForbidden, apperr.AuthAccountBlocked, "account is blocked")
	ErrEmailNotVerified   = apperr.E(apperr.CodeForbidden, apperr.AuthEmailNotVerified, "email not verified")
	ErrUserNotFound       = apperr.E(apperr.CodeNotFound, apperr.NotFound, "user not found")
	// ErrSignedUpWithSocial: a wrong password was tried on an account whose
	// identity came from Google/GitHub; the guidance is to sign in with that
	// provider or reset the password.
	ErrSignedUpWithSocial = apperr.E(apperr.CodeForbidden, apperr.AuthSignedUpWithSocial, "")
)

// Anomaly is one row of the admin session-anomaly report (reference
// auth_session_detect_anomalies semantics).
type Anomaly struct {
	Type    string         // expired_but_active | multi_ip_sessions
	Count   int            // affected sessions (expired) or users (multi_ip)
	Details map[string]any // nil when the anomaly carries no structured detail
}

// MultiIPRow is one GROUP-BY result of the anomaly aggregate: a user whose
// active sessions within the window span more distinct IPs than the SQL
// threshold.
type MultiIPRow struct {
	UserID  string
	IPCount int
}
