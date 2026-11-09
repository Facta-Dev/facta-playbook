package authsignup

import (
	"context"
	"errors"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgconn"
	"github.com/jackc/pgx/v5/pgtype"
	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/<scope>/<app>/internal/gen/db"
)

// Postgres adapts the sqlc-generated queries to this slice's Store port.
type Postgres struct {
	q *db.Queries
}

// NewPostgres builds the adapter over the shared pool.
func NewPostgres(pool *pgxpool.Pool) *Postgres { return &Postgres{q: db.New(pool)} }

var _ Store = (*Postgres)(nil)

func (p *Postgres) GetUserByEmail(ctx context.Context, emailAddr string) (UserRow, bool, error) {
	row, err := p.q.SignupGetUserByEmail(ctx, emailAddr)
	if err == pgx.ErrNoRows {
		return UserRow{}, false, nil
	}
	if err != nil {
		return UserRow{}, false, err
	}
	return UserRow{
		ID:         id(row.ID),
		Email:      row.Email,
		FirstName:  row.FirstName,
		LastName:   row.LastName,
		Role:       row.Role,
		UserType:   row.UserType,
		Language:   row.Language,
		IsActive:   row.IsActive,
		IsVerified: row.IsVerified,
	}, true, nil
}

// CreateUser mirrors the use-case's unique-constraint contract: a lost
// signup race (both saw "unverified", one insert wins) answers as a 409
// EMAIL_TAKEN instead of an unmapped 500.
func (p *Postgres) CreateUser(ctx context.Context, row UserRow) error {
	err := p.q.SignupCreateUser(ctx, db.SignupCreateUserParams{
		ID:        uid(row.ID),
		Email:     row.Email,
		FirstName: row.FirstName,
		LastName:  row.LastName,
		Password:  row.PasswordHash,
		Language:  row.Language,
		UserType:  row.UserType,
	})
	var pgErr *pgconn.PgError
	// "23505" = unique_violation (pgerrcode pkg not worth a dep for one code).
	if errors.As(err, &pgErr) && pgErr.Code == "23505" {
		return ErrEmailTaken
	}
	return err
}

// CreateCompanyProfile seeds the 1:1 companies row keyed by user_id
// (0002_profiles); a PK collision is impossible for a freshly created user,
// so any error here is a real one.
func (p *Postgres) CreateCompanyProfile(ctx context.Context, userID, companyName string) error {
	return p.q.SignupCreateCompanyProfile(ctx, db.SignupCreateCompanyProfileParams{
		UserID: uid(userID), CompanyName: companyName,
	})
}

func (p *Postgres) CreateUserProfile(ctx context.Context, userID, fullName string) error {
	return p.q.SignupCreateUserProfile(ctx, db.SignupCreateUserProfileParams{
		UserID: uid(userID), FullName: fullName,
	})
}

func (p *Postgres) CreateVerification(ctx context.Context, v Verification) error {
	return p.q.SignupCreateVerification(ctx, db.SignupCreateVerificationParams{
		ID:        uid(v.ID),
		UserID:    uid(v.UserID),
		Token:     v.Token,
		ExpiresAt: ts(v.ExpiresAt),
	})
}

func (p *Postgres) LatestVerification(ctx context.Context, userID string) (Verification, bool, error) {
	row, err := p.q.SignupLatestVerification(ctx, uid(userID))
	if err == pgx.ErrNoRows {
		return Verification{}, false, nil
	}
	if err != nil {
		return Verification{}, false, err
	}
	return Verification{
		ID:        id(row.ID),
		UserID:    id(row.UserID),
		Token:     row.Token,
		ExpiresAt: row.ExpiresAt.Time,
		Used:      row.IsUsed,
		Attempts:  int(row.Attempts),
	}, true, nil
}

// UseVerification atomically consumes the code (guards: is_used + token);
// 0 affected rows means a racing request won the code.
func (p *Postgres) UseVerification(ctx context.Context, vID, token string) (bool, error) {
	n, err := p.q.SignupUseVerification(ctx, db.SignupUseVerificationParams{
		ID: uid(vID), Token: token,
	})
	if err != nil {
		return false, err
	}
	return n > 0, nil
}

// BumpVerification records one wrong-code guess.
func (p *Postgres) BumpVerification(ctx context.Context, vID string) error {
	_, err := p.q.SignupBumpVerification(ctx, uid(vID))
	return err
}

func (p *Postgres) SetEmailVerified(ctx context.Context, userID string) error {
	return p.q.SignupSetVerified(ctx, uid(userID))
}

// id converts a pgtype.UUID back to its string form.
func id(u pgtype.UUID) string {
	if !u.Valid {
		return ""
	}
	return uuid.UUID(u.Bytes).String()
}

// uid parses a string id into pgtype.UUID; invalid input yields NULL, and the
// column's primary-key/foreign-key constraints turn that into a query error.
func uid(s string) pgtype.UUID {
	u, err := uuid.Parse(s)
	if err != nil {
		return pgtype.UUID{}
	}
	return pgtype.UUID{Bytes: u, Valid: true}
}

// ts wraps a time for pgtype.Timestamptz.
func ts(t time.Time) pgtype.Timestamptz { return pgtype.Timestamptz{Time: t, Valid: true} }
