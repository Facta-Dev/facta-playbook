package authsignup

import (
	"context"

	"github.com/<scope>/<app>/internal/platform/email"
)

// EmailEnqueuer is this slice's consumer-side port onto the outbox: the
// slice defines the narrow shape it needs; platform/email satisfies it.
type EmailEnqueuer interface {
	Enqueue(ctx context.Context, ms email.Message) error
}

// Store persists users + verification codes (impl: postgres.go).
type Store interface {
	GetUserByEmail(ctx context.Context, emailAddr string) (UserRow, bool, error)
	CreateUser(ctx context.Context, row UserRow) error
	CreateVerification(ctx context.Context, v Verification) error
	LatestVerification(ctx context.Context, userID string) (Verification, bool, error)
	// UseVerification atomically consumes the code; false = raced/taken.
	UseVerification(ctx context.Context, vID, token string) (bool, error)
	// BumpVerification records one wrong-code guess (brute-force cap).
	BumpVerification(ctx context.Context, vID string) error
	SetEmailVerified(ctx context.Context, userID string) error
	// CreateCompanyProfile seeds the 1:1 companies row (user_id PK).
	CreateCompanyProfile(ctx context.Context, userID, companyName string) error
	// CreateUserProfile seeds the 1:1 users_profile row (user_id PK).
	CreateUserProfile(ctx context.Context, userID, fullName string) error
}
