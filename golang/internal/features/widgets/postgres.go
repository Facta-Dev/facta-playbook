package widgets

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

func (p *Postgres) Insert(ctx context.Context, w Widget) error {
	return p.q.WidgetInsert(ctx, db.WidgetInsertParams{
		ID: uid(w.ID), Name: w.Name, Kind: w.Kind, OwnerUserID: uid(w.OwnerUserID),
	})
}

func (p *Postgres) List(ctx context.Context, ownerID string, limit, offset int) ([]Widget, error) {
	rows, err := p.q.WidgetListByOwner(ctx, db.WidgetListByOwnerParams{
		OwnerUserID: uid(ownerID), Limit: int32(limit), Offset: int32(offset),
	})
	if err != nil {
		return nil, err
	}
	out := make([]Widget, 0, len(rows))
	for _, r := range rows {
		out = append(out, conv(r))
	}
	return out, nil
}

func (p *Postgres) Get(ctx context.Context, id, ownerID string) (Widget, bool, error) {
	r, err := p.q.WidgetGetByID(ctx, db.WidgetGetByIDParams{ID: uid(id), OwnerUserID: uid(ownerID)})
	if err == pgx.ErrNoRows {
		return Widget{}, false, nil
	}
	if err != nil {
		return Widget{}, false, err
	}
	return conv(r), true, nil
}

func (p *Postgres) Update(ctx context.Context, id, ownerID string, name, kind string) error {
	return p.q.WidgetUpdate(ctx, db.WidgetUpdateParams{
		ID: uid(id), OwnerUserID: uid(ownerID), Name: name, Kind: kind,
	})
}

func (p *Postgres) Delete(ctx context.Context, id, ownerID string) error {
	return p.q.WidgetDelete(ctx, db.WidgetDeleteParams{ID: uid(id), OwnerUserID: uid(ownerID)})
}

// conv maps a generated row onto the slice model.
func conv(r db.Widget) Widget {
	return Widget{
		ID: id(r.ID), Name: r.Name, Kind: r.Kind,
		OwnerUserID: id(r.OwnerUserID), CreatedAt: r.CreatedAt.Time,
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
