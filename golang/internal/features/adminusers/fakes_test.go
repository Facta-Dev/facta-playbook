package adminusers

import (
	"context"
	"strings"
	"time"

	"github.com/<scope>/<app>/internal/shared/rolex"
)

type fakeStore struct {
	rows    map[string]Row
	audit   []string // recorded action/entity/entityID entries
	nextErr error
}

func (f *fakeStore) List(_ context.Context, limit, offset int) ([]Row, error) {
	out := make([]Row, 0, len(f.rows))
	for _, u := range f.rows {
		out = append(out, u)
	}
	return out, nil
}

func (f *fakeStore) Count(context.Context) (int, error) { return len(f.rows), nil }

func (f *fakeStore) Get(_ context.Context, id string) (Row, bool, error) {
	u, ok := f.rows[id]
	return u, ok, nil
}

// byEmail mirrors the unique email column.
func (f *fakeStore) GetByEmail(_ context.Context, email string) (Row, bool, error) {
	for _, u := range f.rows {
		if strings.EqualFold(u.Email, email) {
			return u, true, nil
		}
	}
	return Row{}, false, nil
}

// UpgradeToAdmin sets role + user_type on the fixed row.
func (f *fakeStore) UpgradeToAdmin(_ context.Context, id string) error {
	u, ok := f.rows[id]
	if ok {
		u.Role, u.UserType = string(rolex.Admin), userTypeAdmin
		f.rows[id] = u
	}
	return nil
}

func (f *fakeStore) Update(_ context.Context, id string, p Patch) error {
	if f.nextErr != nil {
		return f.nextErr
	}
	u, ok := f.rows[id]
	if !ok {
		// DB parity: UPDATE on a missing id affects 0 rows, no error.
		return nil
	}
	if p.IsActive != nil {
		u.IsActive = *p.IsActive
	}
	if p.IsBlocked != nil {
		u.IsBlocked = *p.IsBlocked
	}
	if p.Role != "" {
		u.Role = p.Role
	}
	f.rows[id] = u
	return nil
}

func (f *fakeStore) WriteAudit(_ context.Context, actorID, action, entity, entityID, payload, ip, ua string) error {
	f.audit = append(f.audit, action+"|"+entity+"|"+entityID+"|"+payload+"|"+actorID)
	return nil
}

// newAdminRow seeds one fixed-time user row.
func newAdminRow(id string) Row {
	return Row{
		ID: id, Email: "u" + id + "@ex.com", FirstName: "Ada", LastName: "L",
		Role: string(rolex.User), UserType: "USER", IsActive: true, IsVerified: true,
		Language: "en", CreatedAt: time.Unix(1700, 0),
	}
}
