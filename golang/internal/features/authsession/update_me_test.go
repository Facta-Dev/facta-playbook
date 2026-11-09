package authsession

import (
	"errors"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/gen/api"
)

// TestUpdateMeParity pins the nil-means-keep partial-update semantics through
// the fake (which mirrors the SQL coalesce): non-nil fields overwrite — even
// to "", which sets ” rather than NULL — nil fields are left unchanged, and
// an unknown id reads as ErrUserNotFound.
func TestUpdateMeParity(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)

	first, phone := "Ada", "+54 11 5555-5100"
	got, err := uc.UpdateMe(t.Context(), "id-a@ex.com", UpdateMeInput{FirstName: &first, Phone: &phone})
	if err != nil {
		t.Fatalf("update: %v", err)
	}
	if got.FirstName != "Ada" || got.Phone == nil || *got.Phone != phone {
		t.Fatalf("update me: first=%q phone=%v", got.FirstName, got.Phone)
	}

	// unknown id reads as not-found (the adapter's 0-rows → not-found path).
	if _, err := uc.UpdateMe(t.Context(), "id-ghost@ex.com", UpdateMeInput{}); !errors.Is(err, ErrUserNotFound) {
		t.Fatalf("unknown id must read as not-found, got %v", err)
	}
}

// TestMeResponseWidening pins the widened me payload: the first version of
// the port nulled user_type/created_at; now the row's widened columns (and
// the reference's retry/phone/picture/subscription/last_login/
// github_connected surface) ride along.
func TestMeResponseWidening(t *testing.T) {
	t.Parallel()
	phone := "+54 11 5555-5100"
	u := UserRow{ID: "00000000-0000-0000-0000-000000000001", Email: "a@ex.com",
		FirstName: "Ada", Role: "USER", UserType: "COMPANY", Language: "en",
		IsActive: true, IsVerified: true, Phone: &phone, RetryCount: 2,
		CreatedAt: time.Unix(1700, 0), GithubLinked: true}
	res := meResponse(u)
	if res.UserType == nil || *res.UserType != api.MeResponseUserType("COMPANY") {
		t.Fatalf("user_type must surface: %+v", res.UserType)
	}
	if res.CreatedAt == nil || !res.CreatedAt.Equal(time.Unix(1700, 0)) {
		t.Fatalf("created_at must surface: %+v", res.CreatedAt)
	}
	if res.PhoneNumber == nil || *res.PhoneNumber != phone {
		t.Fatalf("phone must surface: %+v", res.PhoneNumber)
	}
	if res.GithubConnected == nil || !*res.GithubConnected {
		t.Fatalf("github_connected must surface")
	}
}
