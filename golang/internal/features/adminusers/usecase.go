package adminusers

import (
	"context"
	"log/slog"
	"strings"

	"github.com/<scope>/<app>/internal/shared/authz"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// UseCase implements list / get / update with audit trail plus the
// unauthenticated whitelist bootstrap upgrade.
type UseCase struct {
	store     Store
	whitelist map[string]bool
}

// New wires the use-case; allowedEmails is the <APP>_ALLOWED_ADMIN_EMAILS
// whitelist behind the bootstrap upgrade (entries are lowercased; empty =
// endpoint always denies).
func New(store Store, allowedEmails []string) *UseCase {
	wl := make(map[string]bool, len(allowedEmails))
	for _, e := range allowedEmails {
		if e = strings.ToLower(strings.TrimSpace(e)); e != "" {
			wl[e] = true
		}
	}
	return &UseCase{store: store, whitelist: wl}
}

// List returns one page plus the total row count.
func (uc *UseCase) List(ctx context.Context, limit, offset int) ([]Row, int, error) {
	rows, err := uc.store.List(ctx, limit, offset)
	if err != nil {
		return nil, 0, err
	}
	total, err := uc.store.Count(ctx)
	if err != nil {
		return nil, 0, err
	}
	return rows, total, nil
}

// Get fetches a single user.
func (uc *UseCase) Get(ctx context.Context, id string) (Row, error) {
	row, ok, err := uc.store.Get(ctx, id)
	if err != nil {
		return Row{}, err
	}
	if !ok {
		return Row{}, ErrNotFound
	}
	return row, nil
}

// Update applies the patch, validates the role, enforces self-or-subordinate
// (authz: strict rank — an ADMIN row is reachable only by its own, and a
// stale actor token must still outrank the row), and writes the audit entry.
func (uc *UseCase) Update(ctx context.Context, actorID string, actorRole rolex.Role,
	id, ip, ua string, p Patch) (Row, error) {
	if p.IsActive == nil && p.IsBlocked == nil && p.Role == "" {
		return Row{}, ErrNoChanges
	}
	if p.Role != "" && !rolex.Valid(rolex.Role(p.Role)) {
		return Row{}, ErrBadRole
	}
	// Existence first: an unknown id is 404 and never audited.
	target, ok, err := uc.store.Get(ctx, id)
	if err != nil {
		return Row{}, err
	} else if !ok {
		return Row{}, ErrNotFound
	}
	// Self-or-subordinate (shared/authz): an ADMIN cannot manage another
	// ADMIN, and a stale actor role cannot outrank the row's DB role.
	z := authz.Context{SubjectID: actorID, SubjectRole: actorRole,
		TargetID: target.ID, TargetRole: rolex.Role(target.Role)}
	if !z.CanManage() {
		return Row{}, authz.ErrForbidden
	}
	if err := uc.store.Update(ctx, id, p); err != nil {
		return Row{}, err
	}
	// Audit failure must not block the mutation; it is logged loudly so the
	// trail gap is visible.
	if aerr := uc.store.WriteAudit(ctx, actorID, "user.updated", "user", id, payloadOf(p), ip, ua); aerr != nil {
		slog.Error("audit", "action", "user.updated", "user_id", id, "err", aerr)
	}
	return uc.Get(ctx, id)
}

// UpgradeToAdmin is the unauthenticated bootstrap endpoint (reference
// upgrade_to_admin parity): a whitelisted email may promote its own account to
// role+user_type ADMIN. The whitelist is checked before any user lookup, so a
// non-whitelisted caller cannot probe for account existence; a whitelisted
// but unknown email is still 404.
func (uc *UseCase) UpgradeToAdmin(ctx context.Context, rawEmail string) (UpgradeResult, error) {
	email := strings.ToLower(strings.TrimSpace(rawEmail))
	if !uc.whitelist[email] {
		return UpgradeResult{}, ErrNotWhitelisted
	}
	row, ok, err := uc.store.GetByEmail(ctx, email)
	if err != nil {
		return UpgradeResult{}, err
	}
	if !ok {
		return UpgradeResult{}, ErrNotFound
	}
	if row.Role == string(rolex.Admin) && row.UserType == userTypeAdmin {
		return UpgradeResult{Email: email, Message: "User is already an admin"}, nil
	}
	if err := uc.store.UpgradeToAdmin(ctx, row.ID); err != nil {
		return UpgradeResult{}, err
	}
	return UpgradeResult{
		Email:    email,
		Message:  "User '" + email + "' has been upgraded to admin",
		Upgraded: true,
	}, nil
}

// payloadOf renders the update payload for audit_logs (compact JSON).
func payloadOf(p Patch) string {
	s := "{"
	sep := ""
	if p.IsActive != nil {
		s += sep + `"is_active":` + boolJSON(*p.IsActive)
		sep = ","
	}
	if p.IsBlocked != nil {
		s += sep + `"is_blocked":` + boolJSON(*p.IsBlocked)
		sep = ","
	}
	if p.Role != "" {
		s += sep + `"role":"` + p.Role + `"`
	}
	return s + "}"
}

func boolJSON(b bool) string {
	if b {
		return "true"
	}
	return "false"
}
