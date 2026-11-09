package authpassword

import (
	"context"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
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

func (p *Postgres) CountResetsSince(ctx context.Context, email string, since time.Time) (int, error) {
	n, err := p.q.PassCountResetsSince(ctx, db.PassCountResetsSinceParams{Email: email, CreatedAt: ts(since)})
	return int(n), err
}

func (p *Postgres) GetUserByEmail(ctx context.Context, email string) (userRow, bool, error) {
	row, err := p.q.PassGetUserByEmail(ctx, email)
	if err == pgx.ErrNoRows {
		return userRow{}, false, nil
	}
	if err != nil {
		return userRow{}, false, err
	}
	return userRow{
		ID: fromPG(row.ID), Email: row.Email, FirstName: row.FirstName,
		Language: row.Language, IsActive: row.IsActive, IsVerified: row.IsVerified,
	}, true, nil
}

func (p *Postgres) GetUserByID(ctx context.Context, id string) (userRow, bool, error) {
	row, err := p.q.PassGetUserByID(ctx, uid(id))
	if err == pgx.ErrNoRows {
		return userRow{}, false, nil
	}
	if err != nil {
		return userRow{}, false, err
	}
	return userRow{ID: fromPG(row.ID), Email: row.Email, Password: row.Password, Language: row.Language}, true, nil
}

func (p *Postgres) CreateReset(ctx context.Context, r Reset) error {
	return p.q.PassCreateReset(ctx, db.PassCreateResetParams{
		ID: uid(r.ID), UserID: uid(r.UserID), Email: r.Email,
		Token: r.Token, ExpiresAt: ts(r.ExpiresAt),
	})
}

func (p *Postgres) LatestReset(ctx context.Context, email string) (Reset, bool, error) {
	row, err := p.q.PassLatestReset(ctx, email)
	if err == pgx.ErrNoRows {
		return Reset{}, false, nil
	}
	if err != nil {
		return Reset{}, false, err
	}
	return Reset{
		ID: fromPG(row.ID), UserID: fromPG(row.UserID), Email: row.Email,
		Token: row.Token, ExpiresAt: row.ExpiresAt.Time, Used: row.IsUsed,
		Attempts: int(row.Attempts),
	}, true, nil
}

// MarkResetUsed atomically consumes the code (guards: is_used + token);
// 0 affected rows means a racing request won the code.
func (p *Postgres) MarkResetUsed(ctx context.Context, id, token string) (bool, error) {
	n, err := p.q.PassMarkResetUsed(ctx, db.PassMarkResetUsedParams{
		ID: uid(id), Token: token,
	})
	if err != nil {
		return false, err
	}
	return n > 0, nil
}

// BumpReset records one wrong-code guess.
func (p *Postgres) BumpReset(ctx context.Context, id string) error {
	_, err := p.q.PassBumpReset(ctx, uid(id))
	return err
}

func (p *Postgres) SetPasswordAndVerify(ctx context.Context, id, hash string) error {
	return p.q.PassSetPasswordAndVerify(ctx, db.PassSetPasswordAndVerifyParams{ID: uid(id), Password: hash})
}

func (p *Postgres) SetPassword(ctx context.Context, id, hash string) error {
	return p.q.PassSetPassword(ctx, db.PassSetPasswordParams{ID: uid(id), Password: hash})
}

func (p *Postgres) RevokeAllSessions(ctx context.Context, userID string) error {
	return p.q.PassRevokeAllSessions(ctx, uid(userID))
}

func (p *Postgres) RevokeOtherSessions(ctx context.Context, userID, sessionID string) error {
	return p.q.PassRevokeOtherSessions(ctx, db.PassRevokeOtherSessionsParams{UserID: uid(userID), ID: uid(sessionID)})
}

// id converts a pgtype.UUID back to its string form.
func fromPG(u pgtype.UUID) string {
	if !u.Valid {
		return ""
	}
	return uuid.UUID(u.Bytes).String()
}

// uid parses a string id into pgtype.UUID; invalid input yields NULL.
func uid(s string) pgtype.UUID {
	u, err := uuid.Parse(s)
	if err != nil {
		return pgtype.UUID{}
	}
	return pgtype.UUID{Bytes: u, Valid: true}
}

// ts wraps a time for pgtype.Timestamptz.
func ts(t time.Time) pgtype.Timestamptz { return pgtype.Timestamptz{Time: t, Valid: true} }
