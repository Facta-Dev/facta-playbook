package adminusers

import (
	"context"
)

// Store persists admin user reads/updates + the audit trail (impl: postgres.go).
type Store interface {
	List(ctx context.Context, limit, offset int) ([]Row, error)
	Count(ctx context.Context) (int, error)
	Get(ctx context.Context, id string) (Row, bool, error)
	// GetByEmail reads one user by its (unique) email.
	GetByEmail(ctx context.Context, email string) (Row, bool, error)
	Update(ctx context.Context, id string, p Patch) error
	// UpgradeToAdmin sets role + user_type to ADMIN on the user row.
	UpgradeToAdmin(ctx context.Context, id string) error
	// WriteAudit records one audit_logs entry (actor may be unknown).
	WriteAudit(ctx context.Context, actorID, action, entity, entityID, payload, ip, ua string) error
}
