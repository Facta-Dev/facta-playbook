package authpassword

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/email"
)

// EmailEnqueuer is this slice's consumer-side port onto the outbox.
type EmailEnqueuer interface {
	Enqueue(ctx context.Context, ms email.Message) error
}

// Store persists reset codes + password changes (impl: postgres.go).
type Store interface {
	CountResetsSince(ctx context.Context, email string, since time.Time) (int, error)
	GetUserByEmail(ctx context.Context, email string) (userRow, bool, error)
	GetUserByID(ctx context.Context, id string) (userRow, bool, error)
	CreateReset(ctx context.Context, r Reset) error
	LatestReset(ctx context.Context, email string) (Reset, bool, error)
	// MarkResetUsed atomically consumes the code; false = raced/taken.
	MarkResetUsed(ctx context.Context, id, token string) (bool, error)
	// BumpReset records one wrong-code guess (brute-force cap).
	BumpReset(ctx context.Context, id string) error
	SetPasswordAndVerify(ctx context.Context, id, hash string) error
	SetPassword(ctx context.Context, id, hash string) error
	RevokeAllSessions(ctx context.Context, userID string) error
	RevokeOtherSessions(ctx context.Context, userID, sessionID string) error
}

// UserRow re-exports the projection under an exported name: the godog
// acceptance harness implements this Store port from another package, and a
// method signature there cannot spell the unexported type.
type UserRow = userRow

// userRow is the by-email/by-ID user projection (mirrors the gen rows).
type userRow struct {
	ID         string
	Email      string
	FirstName  string
	Language   string
	IsActive   bool
	IsVerified bool
	Password   string
}
