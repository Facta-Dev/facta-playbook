package adminusers

import (
	"context"

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

func (p *Postgres) List(ctx context.Context, limit, offset int) ([]Row, error) {
	rows, err := p.q.AdminListUsers(ctx, db.AdminListUsersParams{Limit: int32(limit), Offset: int32(offset)})
	if err != nil {
		return nil, err
	}
	out := make([]Row, 0, len(rows))
	for _, r := range rows {
		out = append(out, Row{
			ID: fromPG(r.ID), Email: r.Email,
			FirstName: r.FirstName, LastName: r.LastName,
			Role: r.Role, UserType: r.UserType,
			IsActive: r.IsActive, IsBlocked: r.IsBlocked, IsVerified: r.IsVerified,
			Language: r.Language, CreatedAt: r.CreatedAt.Time,
		})
	}
	return out, nil
}

func (p *Postgres) Count(ctx context.Context) (int, error) {
	n, err := p.q.AdminCountUsers(ctx)
	return int(n), err
}

func (p *Postgres) Get(ctx context.Context, id string) (Row, bool, error) {
	r, err := p.q.AdminGetUser(ctx, uid(id))
	if err == pgx.ErrNoRows {
		return Row{}, false, nil
	}
	if err != nil {
		return Row{}, false, err
	}
	return Row{
		ID: fromPG(r.ID), Email: r.Email,
		FirstName: r.FirstName, LastName: r.LastName,
		Role: r.Role, UserType: r.UserType,
		IsActive: r.IsActive, IsBlocked: r.IsBlocked, IsVerified: r.IsVerified,
		Language: r.Language, CreatedAt: r.CreatedAt.Time, LastLogin: r.LastLogin.Time,
	}, true, nil
}

// GetByEmail mirrors Get on the unique email column.
func (p *Postgres) GetByEmail(ctx context.Context, email string) (Row, bool, error) {
	r, err := p.q.AdminGetUserByEmail(ctx, email)
	if err == pgx.ErrNoRows {
		return Row{}, false, nil
	}
	if err != nil {
		return Row{}, false, err
	}
	return Row{
		ID: fromPG(r.ID), Email: r.Email,
		FirstName: r.FirstName, LastName: r.LastName,
		Role: r.Role, UserType: r.UserType,
		IsActive: r.IsActive, IsBlocked: r.IsBlocked, IsVerified: r.IsVerified,
		Language: r.Language, CreatedAt: r.CreatedAt.Time, LastLogin: r.LastLogin.Time,
	}, true, nil
}

// UpgradeToAdmin moves role + user_type to ADMIN (bootstrap whitelist route).
func (p *Postgres) UpgradeToAdmin(ctx context.Context, id string) error {
	return p.q.AdminUpgradeToAdmin(ctx, uid(id))
}

func (p *Postgres) Update(ctx context.Context, id string, patch Patch) error {
	return p.q.AdminUpdateUser(ctx, db.AdminUpdateUserParams{
		IsActive:  boolPtr(patch.IsActive),
		IsBlocked: boolPtr(patch.IsBlocked),
		Role:      textPtr(patch.Role),
		ID:        uid(id),
	})
}

func (p *Postgres) WriteAudit(ctx context.Context, actorID, action, entity, entityID, payload, ip, ua string) error {
	actor := pgtype.UUID{}
	if actorID != "" {
		actor = uid(actorID)
	}
	return p.q.AdminWriteAudit(ctx, db.AdminWriteAuditParams{
		ID:          uid(uuid.NewString()),
		ActorUserID: actor,
		Action:      action,
		Entity:      entity,
		EntityID:    pgtype.Text{String: entityID, Valid: entityID != ""},
		Payload:     []byte(payload),
		IpAddress:   pgtype.Text{String: ip, Valid: ip != ""},
		UserAgent:   pgtype.Text{String: ua, Valid: ua != ""},
	})
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

// boolPtr maps a *bool onto the sqlc.narg column.
func boolPtr(b *bool) pgtype.Bool {
	if b == nil {
		return pgtype.Bool{}
	}
	return pgtype.Bool{Bool: *b, Valid: true}
}

// textPtr maps an optional string onto the sqlc.narg column.
func textPtr(s string) pgtype.Text {
	if s == "" {
		return pgtype.Text{}
	}
	return pgtype.Text{String: s, Valid: true}
}
