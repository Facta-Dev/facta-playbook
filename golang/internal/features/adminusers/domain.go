// Package adminusers owns the admin user-management endpoints: paginated
// list, single fetch and partial update behind RequireRole(rolex.Admin),
// with an audit_logs trail written on every mutation.
package adminusers

import (
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

// Row is the admin view of one user.
type Row struct {
	ID         string
	Email      string
	FirstName  string
	LastName   string
	Role       string
	UserType   string
	IsActive   bool
	IsBlocked  bool
	IsVerified bool
	Language   string
	CreatedAt  time.Time
	LastLogin  time.Time
}

// Patch is the nullable update applied to a user.
type Patch struct {
	IsActive  *bool
	IsBlocked *bool
	Role      string // "" leaves the role untouched
}

// UpgradeResult is the outcome of the unauthenticated whitelist upgrade.
type UpgradeResult struct {
	Email    string
	Message  string
	Upgraded bool // false reads as "the account already was an admin"
}

// user_type ADMIN; slice-local because cross-slice imports are forbidden —
// authsignup owns the other user_type constants for provisioning.
const userTypeAdmin = "ADMIN"

// Slice errors.
var (
	ErrNotFound  = apperr.E(apperr.CodeNotFound, apperr.NotFound, "user not found")
	ErrBadRole   = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "unknown role")
	ErrNoChanges = apperr.E(apperr.CodeUnprocessable, apperr.Validation, "empty update")
	// ErrNotWhitelisted: the bootstrap upgrade denies any email absent from
	// <APP>_ALLOWED_ADMIN_EMAILS before a user row is even looked up.
	ErrNotWhitelisted = apperr.E(apperr.CodeForbidden, apperr.AuthForbidden, "email is not authorized to upgrade to admin")
)
