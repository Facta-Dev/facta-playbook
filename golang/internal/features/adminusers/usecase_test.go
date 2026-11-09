package adminusers

import (
	"testing"

	"github.com/<scope>/<app>/internal/shared/rolex"
)

func TestAdminListReturnsTotal(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Row{"u-1": newAdminRow("u-1")}, audit: []string{}}
	uc := New(store, nil)

	rows, total, err := uc.List(t.Context(), 20, 0)
	if err != nil {
		t.Fatalf("list: %v", err)
	}
	if total != 1 || len(rows) != 1 || rows[0].ID != "u-1" {
		t.Fatalf("bad page: %d rows, total %d", len(rows), total)
	}
	if got, ok, _ := store.Get(t.Context(), "u-1"); !ok || got.Email != newAdminRow("u-1").Email {
		t.Fatalf("get: %+v ok=%v", got, ok)
	}
	if _, err := uc.Get(t.Context(), "ghost"); err == nil {
		t.Fatal("missing user must 404")
	}
}

func TestAdminUpdateValidatesRoleAndAudits(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Row{"u-1": newAdminRow("u-1")}, audit: []string{}}
	uc := New(store, nil)

	if _, err := uc.Update(t.Context(), "admin-1", rolex.Admin, "u-1", "9.9.9.9", "ua", Patch{Role: "WIZARD"}); err == nil {
		t.Fatal("unknown role must be rejected")
	}
	if _, err := uc.Update(t.Context(), "admin-1", rolex.Admin, "u-1", "9.9.9.9", "ua", Patch{}); err == nil {
		t.Fatal("empty patch must be rejected")
	}
	if _, err := uc.Update(t.Context(), "admin-1", rolex.Admin, "ghost", "9.9.9.9", "ua", Patch{Role: "MODERATOR"}); err == nil {
		t.Fatal("missing user must be rejected")
	}

	row, err := uc.Update(t.Context(), "admin-1", rolex.Admin, "u-1", "9.9.9.9", "ua", Patch{Role: "MODERATOR"})
	if err != nil {
		t.Fatalf("update: %v", err)
	}
	got, _, _ := store.Get(t.Context(), "u-1")
	if got.Role != "MODERATOR" || row.Role != "MODERATOR" {
		t.Fatalf("role not applied: %+v / %+v", got, row)
	}
	if got := store.audit; len(got) != 1 {
		t.Fatalf("one audit entry expected, got %d", len(got))
	} else if got[0] != "user.updated|user|u-1|"+`{"role":"MODERATOR"}`+"|admin-1" {
		t.Fatalf("bad audit entry: %q", got[0])
	}
}

func TestAdminUpdateNullableBooleans(t *testing.T) {
	t.Parallel()
	store := &fakeStore{rows: map[string]Row{"u-1": newAdminRow("u-1")}, audit: []string{}}
	uc := New(store, nil)

	off := false
	if _, err := uc.Update(t.Context(), "admin-1", rolex.Admin, "u-1", "", "", Patch{IsActive: &off}); err != nil {
		t.Fatalf("update: %v", err)
	}
	if got, _, _ := store.Get(t.Context(), "u-1"); got.IsActive {
		t.Fatal("is_active=false not applied")
	}
	if got := store.audit; len(got) != 1 || got[0] != "user.updated|user|u-1|"+`{"is_active":false}`+"|admin-1" {
		t.Fatalf("bad audit: %v", store.audit)
	}
}
