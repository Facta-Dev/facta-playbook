package authsession

import (
	"context"
	"sort"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

type fakeStore struct {
	now      time.Time
	users    map[string]UserRow // keyed by email
	byID     map[string]UserRow
	sessions map[string]SessionRow // keyed by refresh token
}

func newFakeStore(now time.Time) *fakeStore {
	u := UserRow{ID: "id-a@ex.com", Email: "a@ex.com", PasswordHash: "",
		Role: "USER", Language: "en", IsActive: true, IsVerified: true, IsBlocked: false}
	return &fakeStore{
		now:      now,
		users:    map[string]UserRow{"a@ex.com": u},
		byID:     map[string]UserRow{u.ID: u},
		sessions: map[string]SessionRow{},
	}
}

func (f *fakeStore) GetUserByEmail(_ context.Context, email string) (UserRow, bool, error) {
	u, ok := f.users[email]
	return u, ok, nil
}

func (f *fakeStore) GetUserByID(_ context.Context, id string) (UserRow, bool, error) {
	u, ok := f.byID[id]
	return u, ok, nil
}

func (f *fakeStore) AfterFailedLogin(_ context.Context, userID string, lockAt int) (bool, error) {
	blocked := false
	f.mut(userID, func(u UserRow) UserRow {
		u.RetryCount++
		blocked = u.RetryCount >= lockAt
		u.IsBlocked = blocked
		return u
	})
	return blocked, nil
}

func (f *fakeStore) AfterSuccessfulLogin(_ context.Context, userID string) error {
	f.mut(userID, func(u UserRow) UserRow { u.RetryCount = 0; return u })
	return nil
}

// mut rewrites the user in both maps.
func (f *fakeStore) mut(userID string, fn func(UserRow) UserRow) {
	for k, u := range f.users {
		if u.ID == userID {
			f.users[k] = fn(u)
		}
	}
	for k, u := range f.byID {
		if u.ID == userID {
			f.byID[k] = fn(u)
		}
	}
}

func (f *fakeStore) CreateSession(_ context.Context, s SessionRow) error {
	f.sessions[s.RefreshToken] = s
	return nil
}

func (f *fakeStore) ActiveSessionByToken(_ context.Context, token string) (SessionUser, bool, error) {
	s, ok := f.sessions[token]
	if !ok || !s.IsActive || !s.ExpiresAt.After(f.now) {
		return SessionUser{}, false, nil
	}
	u := f.byID[s.UserID]
	return SessionUser{SessionRow: s, Email: u.Email, Role: u.Role,
		UserActive: u.IsActive, IsBlocked: u.IsBlocked, IsVerified: u.IsVerified}, true, nil
}

func (f *fakeStore) DeactivateSession(_ context.Context, sessionID string) (bool, error) {
	for k, s := range f.sessions {
		if s.ID == sessionID {
			was := s.IsActive
			s.IsActive = false
			f.sessions[k] = s
			return was, nil
		}
	}
	return false, nil
}

// RotateSession mirrors the SQL: matching id + old token + active + not
// expired claims the row in place, re-keying the map by the new token.
func (f *fakeStore) RotateSession(_ context.Context, sessionID, oldToken string, s SessionRow) (bool, error) {
	for k, cur := range f.sessions {
		if cur.ID == sessionID && cur.RefreshToken == oldToken && cur.IsActive && cur.ExpiresAt.After(f.now) {
			s.ID, s.CreatedAt, s.ExpiresAt, s.LastActivity = cur.ID, cur.CreatedAt, cur.ExpiresAt, f.now
			delete(f.sessions, k)
			f.sessions[s.RefreshToken] = s
			return true, nil
		}
	}
	return false, nil
}

func (f *fakeStore) DeactivateByTokenForUser(_ context.Context, userID, token string) error {
	for k, s := range f.sessions {
		if s.RefreshToken == token && s.UserID == userID {
			s.IsActive = false
			f.sessions[k] = s
		}
	}
	return nil
}

func (f *fakeStore) ListSessionsByUser(_ context.Context, userID string, activeOnly bool) ([]SessionRow, error) {
	var out []SessionRow
	for _, s := range f.sessions {
		if s.UserID != userID {
			continue
		}
		if activeOnly && !s.IsActive {
			continue
		}
		out = append(out, s)
	}
	// Mirror the SQL ordering (map iteration is random).
	sort.Slice(out, func(i, j int) bool { return out[i].LastActivity.After(out[j].LastActivity) })
	return out, nil
}

func (f *fakeStore) CurrentSession(_ context.Context, userID string) (SessionRow, bool, error) {
	var best SessionRow
	found := false
	for _, s := range f.sessions {
		if s.UserID == userID && s.IsActive && (!found || s.LastActivity.After(best.LastActivity)) {
			best, found = s, true
		}
	}
	return best, found, nil
}

func (f *fakeStore) RevokeSessionForUser(_ context.Context, userID, sessionID string) (bool, error) {
	for k, s := range f.sessions {
		if s.ID == sessionID && s.UserID == userID && s.IsActive {
			s.IsActive = false
			f.sessions[k] = s
			return true, nil
		}
	}
	return false, nil
}

func (f *fakeStore) RevokeAllForUser(_ context.Context, userID string) (int, error) {
	n := 0
	for k, s := range f.sessions {
		if s.UserID == userID && s.IsActive {
			s.IsActive = false
			f.sessions[k] = s
			n++
		}
	}
	return n, nil
}

func (f *fakeStore) CountExpiredActive(_ context.Context) (int, error) {
	n := 0
	for _, s := range f.sessions {
		if s.IsActive && !s.ExpiresAt.After(f.now) {
			n++
		}
	}
	return n, nil
}

// MultiIPSessions mirrors the SQL aggregate: active sessions within the
// window grouped per user by distinct IP, > 3 IPs is an anomaly row.
func (f *fakeStore) MultiIPSessions(_ context.Context, since time.Time) ([]MultiIPRow, error) {
	ips := map[string]map[string]bool{}
	for _, s := range f.sessions {
		if !s.IsActive || s.LastActivity.Before(since) {
			continue
		}
		if ips[s.UserID] == nil {
			ips[s.UserID] = map[string]bool{}
		}
		ips[s.UserID][s.IPAddress] = true
	}
	var out []MultiIPRow
	for user, set := range ips {
		if len(set) > 3 {
			out = append(out, MultiIPRow{UserID: user, IPCount: len(set)})
		}
	}
	return out, nil
}

// UpdateMe mirrors the SQL: non-nil pointers overwrite — even to "" — nil
// pointers keep the current value; unknown id reads as not-found.
func (f *fakeStore) UpdateMe(_ context.Context, userID string, in UpdateMeInput) (UserRow, bool, error) {
	found := false
	var out UserRow
	f.mut(userID, func(u UserRow) UserRow {
		found = true
		if in.FirstName != nil {
			u.FirstName = *in.FirstName
		}
		if in.LastName != nil {
			u.LastName = *in.LastName
		}
		if in.Phone != nil {
			u.Phone = in.Phone
		}
		if in.Picture != nil {
			u.Picture = in.Picture
		}
		if in.Language != nil {
			u.Language = *in.Language
		}
		if in.Address != nil {
			u.Address = in.Address
		}
		if in.Postal != nil {
			u.Postal = in.Postal
		}
		out = u
		return u
	})
	return out, found, nil
}

type stubMinter struct{ calls int }

func (s *stubMinter) Mint(id authx.Identity, ttl time.Duration) (string, time.Time, error) {
	s.calls++
	return "access-token." + id.JTI, time.Now().Add(ttl), nil
}

// newUC builds the use-case with a fixed clock and default TTLs.
func newUC(store Store, minter TokenMinter, now time.Time) *UseCase {
	return New(store, minter, clockx.Fixed(now), "pepper", 15*time.Minute, 720*time.Hour, 5)
}

// correctPass is the password the seeded user hashes to in gate tests.
const correctPass = "correct-horse"

// seedHash gives the seeded user a real password hash so the login tests can
// exercise the post-password ordering.
func seedHash(t *testing.T, store *fakeStore) {
	t.Helper()
	u := store.users["a@ex.com"]
	u.PasswordHash = hashForTest(t, correctPass, "pepper")
	store.users["a@ex.com"] = u
}

// hashForTest produces a real bcrypt hash for a known password.
func hashForTest(t *testing.T, password, pepper string) string {
	t.Helper()
	h, err := authx.HashPassword(password, pepper)
	if err != nil {
		t.Fatalf("hash: %v", err)
	}
	return h
}
func setBlock(v bool) func(UserRow) UserRow {
	return func(u UserRow) UserRow { u.IsBlocked = v; return u }
}
func setActive(v bool) func(UserRow) UserRow {
	return func(u UserRow) UserRow { u.IsActive = v; return u }
}
func setVerified(v bool) func(UserRow) UserRow {
	return func(u UserRow) UserRow { u.IsVerified = v; return u }
}
