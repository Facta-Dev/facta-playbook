package authsession

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

func (p *Postgres) GetUserByEmail(ctx context.Context, emailAddr string) (UserRow, bool, error) {
	row, err := p.q.SessionLoginGetUserByEmail(ctx, emailAddr)
	if err == pgx.ErrNoRows {
		return UserRow{}, false, nil
	}
	if err != nil {
		return UserRow{}, false, err
	}
	return UserRow{
		ID:           id(row.ID),
		Email:        row.Email,
		PasswordHash: row.Password,
		Role:         row.Role,
		Language:     row.Language,
		IsActive:     row.IsActive,
		IsBlocked:    row.IsBlocked,
		IsVerified:   row.IsVerified,
		RetryCount:   int(row.RetryCount),
		GoogleLinked: row.GoogleID.Valid,
		GithubLinked: row.GithubID.Valid,
	}, true, nil
}

func (p *Postgres) GetUserByID(ctx context.Context, userID string) (UserRow, bool, error) {
	row, err := p.q.SessionUserByID(ctx, uid(userID))
	if err == pgx.ErrNoRows {
		return UserRow{}, false, nil
	}
	if err != nil {
		return UserRow{}, false, err
	}
	return UserRow{
		ID:           id(row.ID),
		Email:        row.Email,
		FirstName:    row.FirstName,
		LastName:     row.LastName,
		Role:         row.Role,
		UserType:     row.UserType,
		Language:     row.Language,
		Phone:        textPtr(row.PhoneNumber),
		Picture:      textPtr(row.ProfilePicture),
		Subscription: textPtr(row.Subscription),
		Address:      textPtr(row.Address),
		Postal:       textPtr(row.PostalCode),
		IsActive:     row.IsActive,
		IsBlocked:    row.IsBlocked,
		IsVerified:   row.IsVerified,
		RetryCount:   int(row.RetryCount),
		LastLogin:    tsPtr(row.LastLogin),
		CreatedAt:    row.CreatedAt.Time,
		UpdatedAt:    row.UpdatedAt.Time,
		GoogleLinked: row.GoogleLinked,
		GithubLinked: row.GithubLinked,
	}, true, nil
}

// UpdateMe applies the partial profile update and re-reads the profile
// (reference update-then-GetMe parity): 0 affected rows = unknown id.
func (p *Postgres) UpdateMe(ctx context.Context, userID string, in UpdateMeInput) (UserRow, bool, error) {
	tag, err := p.q.SessionUpdateMe(ctx, db.SessionUpdateMeParams{
		FirstName:      textSet(in.FirstName),
		LastName:       textSet(in.LastName),
		PhoneNumber:    textSet(in.Phone),
		ProfilePicture: textSet(in.Picture),
		Language:       textSet(in.Language),
		Address:        textSet(in.Address),
		PostalCode:     textSet(in.Postal),
		UserID:         uid(userID),
	})
	if err != nil {
		return UserRow{}, false, err
	}
	if tag == 0 {
		return UserRow{}, false, nil
	}
	return p.GetUserByID(ctx, userID)
}

func (p *Postgres) AfterFailedLogin(ctx context.Context, userID string, lockAt int) (bool, error) {
	return p.q.SessionAfterFailedLogin(ctx, db.SessionAfterFailedLoginParams{
		ID: uid(userID), LockAt: int32(lockAt),
	})
}

func (p *Postgres) AfterSuccessfulLogin(ctx context.Context, userID string) error {
	return p.q.SessionAfterSuccessfulLogin(ctx, uid(userID))
}

func (p *Postgres) CreateSession(ctx context.Context, s SessionRow) error {
	return p.q.SessionCreate(ctx, db.SessionCreateParams{
		ID:           uid(s.ID),
		UserID:       uid(s.UserID),
		RefreshToken: s.RefreshToken,
		IpAddress:    text(s.IPAddress),
		UserAgent:    text(s.UserAgent),
		ExpiresAt:    ts(s.ExpiresAt),
	})
}

func (p *Postgres) ActiveSessionByToken(ctx context.Context, token string) (SessionUser, bool, error) {
	row, err := p.q.SessionGetActiveByToken(ctx, token)
	if err == pgx.ErrNoRows {
		return SessionUser{}, false, nil
	}
	if err != nil {
		return SessionUser{}, false, err
	}
	inner := SessionRow{
		ID:           id(row.SessionID),
		UserID:       id(row.UserID),
		RefreshToken: row.RefreshToken,
		ExpiresAt:    row.ExpiresAt.Time,
		IsActive:     row.IsActive,
	}
	return SessionUser{SessionRow: inner, Email: row.Email, Role: row.Role,
		UserActive: row.UserIsActive, IsBlocked: row.IsBlocked, IsVerified: row.IsVerified}, true, nil
}

// DeactivateSession claims the refresh-row outright (bad account state and
// revocation paths).
func (p *Postgres) DeactivateSession(ctx context.Context, sessionID string) (bool, error) {
	tag, err := p.q.SessionDeactivate(ctx, uid(sessionID))
	if err != nil {
		return false, err
	}
	return tag > 0, nil
}

// RotateSession rotates the refresh token in place: the session keeps its id
// and expires_at (fixed absolute lifetime), only token/ip/ua/last_activity
// change. false means the old token lost the compare-and-swap (replay).
func (p *Postgres) RotateSession(ctx context.Context, sessionID, oldToken string, s SessionRow) (bool, error) {
	tag, err := p.q.SessionRotateTokens(ctx, db.SessionRotateTokensParams{
		RefreshToken:   s.RefreshToken,
		IpAddress:      text(s.IPAddress),
		UserAgent:      text(s.UserAgent),
		ID:             uid(sessionID),
		RefreshToken_2: oldToken,
	})
	if err != nil {
		return false, err
	}
	return tag > 0, nil
}

func (p *Postgres) DeactivateByTokenForUser(ctx context.Context, userID, token string) error {
	return p.q.SessionDeactivateByTokenForUser(ctx, db.SessionDeactivateByTokenForUserParams{
		RefreshToken: token,
		UserID:       uid(userID),
	})
}

func (p *Postgres) ListSessionsByUser(ctx context.Context, userID string, activeOnly bool) ([]SessionRow, error) {
	rows, err := p.q.SessionListByUser(ctx, db.SessionListByUserParams{
		UserID: uid(userID), ActiveOnly: activeOnly,
	})
	if err != nil {
		return nil, err
	}
	out := make([]SessionRow, 0, len(rows))
	for _, r := range rows {
		out = append(out, sessionOf(r))
	}
	return out, nil
}

func (p *Postgres) CurrentSession(ctx context.Context, userID string) (SessionRow, bool, error) {
	row, err := p.q.SessionCurrentByUser(ctx, uid(userID))
	if err == pgx.ErrNoRows {
		return SessionRow{}, false, nil
	}
	if err != nil {
		return SessionRow{}, false, err
	}
	return sessionOf(row), true, nil
}

func (p *Postgres) RevokeSessionForUser(ctx context.Context, userID, sessionID string) (bool, error) {
	tag, err := p.q.SessionRevokeForUser(ctx, db.SessionRevokeForUserParams{
		UserID: uid(userID), ID: uid(sessionID),
	})
	if err != nil {
		return false, err
	}
	return tag > 0, nil
}

func (p *Postgres) RevokeAllForUser(ctx context.Context, userID string) (int, error) {
	tag, err := p.q.SessionRevokeAllForUser(ctx, uid(userID))
	if err != nil {
		return 0, err
	}
	return int(tag), nil
}

func (p *Postgres) CountExpiredActive(ctx context.Context) (int, error) {
	n, err := p.q.SessionCountExpiredActive(ctx)
	return int(n), err
}

func (p *Postgres) MultiIPSessions(ctx context.Context, since time.Time) ([]MultiIPRow, error) {
	rows, err := p.q.SessionAnomalyMultiIP(ctx, ts(since))
	if err != nil {
		return nil, err
	}
	out := make([]MultiIPRow, 0, len(rows))
	for _, r := range rows {
		out = append(out, MultiIPRow{UserID: id(r.UserID), IPCount: int(r.IpCount)})
	}
	return out, nil
}

// sessionOf maps the generated table row onto the domain row.
func sessionOf(s db.Session) SessionRow {
	return SessionRow{
		ID:           id(s.ID),
		UserID:       id(s.UserID),
		RefreshToken: s.RefreshToken,
		IPAddress:    s.IpAddress.String,
		UserAgent:    s.UserAgent.String,
		ExpiresAt:    s.ExpiresAt.Time,
		IsActive:     s.IsActive,
		CreatedAt:    s.CreatedAt.Time,
		LastActivity: s.LastActivity.Time,
	}
}

// id converts a pgtype.UUID back to its string form.
func id(u pgtype.UUID) string {
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

// text wraps a nullable string column.
func text(s string) pgtype.Text { return pgtype.Text{String: s, Valid: s != ""} }

// ts wraps a time for pgtype.Timestamptz.
func ts(t time.Time) pgtype.Timestamptz { return pgtype.Timestamptz{Time: t, Valid: true} }

// textSet wraps an optional me-update pointer: nil → NULL (the SQL coalesce
// then keeps the current value), present → the string even when empty (”
// must be settable — reference parity).
func textSet(s *string) pgtype.Text {
	if s == nil {
		return pgtype.Text{}
	}
	return pgtype.Text{String: *s, Valid: true}
}

// textPtr unwraps a nullable varchar/text column onto the domain pointer
// (nil = NULL, present = the string even when empty).
func textPtr(t pgtype.Text) *string {
	if !t.Valid {
		return nil
	}
	s := t.String
	return &s
}

// tsPtr unwraps a nullable timestamptz column onto the domain pointer.
func tsPtr(t pgtype.Timestamptz) *time.Time {
	if !t.Valid {
		return nil
	}
	tt := t.Time
	return &tt
}
