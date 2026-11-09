// Package authoauth owns the social logins: Google One-Tap (a verified ID
// token provisions-or-links the account, then issues the same login shape
// as the email flow) and GitHub OAuth (authorize redirect + callback that
// ALWAYS answers 302 — tokens ride the URL fragment on success and
// ?error=/error_description= on failure, never JSON).
package authoauth

import (
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// User is the minimal user projection this slice needs.
type User struct {
	ID         string
	Email      string
	FirstName  string
	LastName   string
	Password   string // only on provisioning: unusable random secret, never read back
	Picture    string
	Role       string
	UserType   string
	Language   string
	GoogleID   string
	GithubID   string
	IsActive   bool
	IsBlocked  bool
	IsVerified bool
}

// SessionRow is one durable refresh session (mirrors the sessions table).
type SessionRow struct {
	ID           string
	UserID       string
	RefreshToken string
	IPAddress    string
	UserAgent    string
	ExpiresAt    time.Time
	IsActive     bool
}

// Result is the login-shape payload (same fields the email login returns).
type Result struct {
	AccessToken  string
	RefreshToken string
	ExpiresAt    time.Time
}

// LinkStatePrefix marks a callback state as account-linking ("link:<jwt>");
// anything else is a plain login.
const LinkStatePrefix = "link:"

// Frontend redirect labels (reference parity).
const (
	errGithubNotConfig  = "github_not_configured"
	errAuthFailed       = "auth_failed"
	errNoCode           = "no_code"
	linkSuccessRedirect = "/user/profile?tab=overview&github_linked=1"
	authSuccessSuffix   = "/auth/success"
)

// Errors of this slice. Google sign-in answers 401 like bad credentials;
// a disabled GitHub provider is an operator problem and answers 403 like
// the admin surface (openapi: both github routes document the 403 Envelope).
// Inactive/blocked match the email login's statuses (authsession parity).
var (
	ErrInvalidGoogleToken  = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "invalid google token")
	ErrGoogleEmailLinked   = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "email is linked to another account")
	ErrGithubAlreadyLinked = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "github account is already linked to another user")
	ErrGoogleNotConfigured = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidCredentials, "google sign-in is not configured")
	ErrGithubNotConfigured = apperr.E(apperr.CodeForbidden, apperr.AuthForbidden, "github sign-in is not configured")
	ErrInvalidLinkState    = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "invalid or expired link")
	ErrAccountInactive     = apperr.E(apperr.CodeForbidden, apperr.AuthAccountInactive, "account is inactive")
	ErrAccountBlocked      = apperr.E(apperr.CodeForbidden, apperr.AuthAccountBlocked, "account is blocked")
	ErrUserNotFound        = apperr.E(apperr.CodeNotFound, apperr.NotFound, "user not found")
	ErrValidation          = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request")
)
