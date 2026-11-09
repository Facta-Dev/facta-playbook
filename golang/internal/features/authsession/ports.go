package authsession

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
)

// TokenMinter is the access-token issuance port (platform/authx satisfies it).
type TokenMinter interface {
	Mint(id authx.Identity, ttl time.Duration) (token string, exp time.Time, err error)
}

// Store persists sessions + reads users (impl: postgres.go).
type Store interface {
	GetUserByEmail(ctx context.Context, email string) (UserRow, bool, error)
	GetUserByID(ctx context.Context, id string) (UserRow, bool, error)
	// UpdateMe applies the non-nil fields (nil = leave unchanged; '' sets
	// the empty string, never NULL) and re-reads the profile — reference
	// update-then-GetMe parity. false when the user does not exist.
	UpdateMe(ctx context.Context, userID string, in UpdateMeInput) (UserRow, bool, error)
	// AfterFailedLogin increments SQL-side and returns whether the user
	// just crossed the lock threshold.
	AfterFailedLogin(ctx context.Context, userID string, lockAt int) (blocked bool, err error)
	AfterSuccessfulLogin(ctx context.Context, userID string) error
	CreateSession(ctx context.Context, s SessionRow) error
	ActiveSessionByToken(ctx context.Context, token string) (SessionUser, bool, error)
	// DeactivateSession claims the refresh-row outright: false when the
	// session was already deactivated/revoked.
	DeactivateSession(ctx context.Context, sessionID string) (bool, error)
	// RotateSession rotates the refresh token in place (the row keeps its id
	// and expires_at — fixed absolute lifetime, reference update_tokens
	// parity): false when the old token lost the compare-and-swap (replay).
	RotateSession(ctx context.Context, sessionID, oldToken string, s SessionRow) (bool, error)
	DeactivateByTokenForUser(ctx context.Context, userID, token string) error

	// Session-management views (own + admin) and the anomaly aggregates.
	ListSessionsByUser(ctx context.Context, userID string, activeOnly bool) ([]SessionRow, error)
	// CurrentSession is the user's most recent active session by
	// last_activity.
	CurrentSession(ctx context.Context, userID string) (SessionRow, bool, error)
	// RevokeSessionForUser claims one active session of the user
	// (compare-and-swap on is_active): false when the id is not one of the
	// user's active sessions.
	RevokeSessionForUser(ctx context.Context, userID, sessionID string) (bool, error)
	// RevokeAllForUser deactivates every active session of the user and
	// returns how many were claimed.
	RevokeAllForUser(ctx context.Context, userID string) (int, error)
	CountExpiredActive(ctx context.Context) (int, error)
	MultiIPSessions(ctx context.Context, since time.Time) ([]MultiIPRow, error)
}
