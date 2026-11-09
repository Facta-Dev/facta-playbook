package authpassword

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/email"
)

// fakeStore is an in-memory Store.
type fakeStore struct {
	now      time.Time
	users    map[string]userRow // keyed by id AND by email
	resets   map[string]Reset   // keyed by id
	order    []string           // creation order of reset ids
	sessions map[string]bool    // session id -> active
}

func newFakeStore(now time.Time) *fakeStore {
	u := userRow{ID: "u-1", Email: "a@ex.com", FirstName: "Ada", Language: "es",
		IsActive: true, IsVerified: false}
	return &fakeStore{now: now, users: map[string]userRow{u.ID: u, u.Email: u},
		resets: map[string]Reset{}, sessions: map[string]bool{}}
}

func (f *fakeStore) CountResetsSince(_ context.Context, email string, since time.Time) (int, error) {
	n := 0
	for _, id := range f.order {
		r := f.resets[id]
		if r.Email == email && r.CreatedAt.After(since) {
			n++
		}
	}
	return n, nil
}

func (f *fakeStore) GetUserByEmail(_ context.Context, email string) (userRow, bool, error) {
	u, ok := f.users[email]
	return u, ok, nil
}

func (f *fakeStore) GetUserByID(_ context.Context, id string) (userRow, bool, error) {
	u, ok := f.users[id]
	return u, ok, nil
}

func (f *fakeStore) CreateReset(_ context.Context, r Reset) error {
	r.CreatedAt = f.now
	f.resets[r.ID] = r
	f.order = append(f.order, r.ID)
	return nil
}

func (f *fakeStore) LatestReset(_ context.Context, email string) (Reset, bool, error) {
	for i := len(f.order) - 1; i >= 0; i-- {
		r := f.resets[f.order[i]]
		if r.Email == email {
			return r, true, nil
		}
	}
	return Reset{}, false, nil
}

func (f *fakeStore) MarkResetUsed(_ context.Context, id, token string) (bool, error) {
	r := f.resets[id]
	if r.Used || r.Token != token {
		return false, nil
	}
	r.Used = true
	f.resets[id] = r
	return true, nil
}

func (f *fakeStore) BumpReset(_ context.Context, id string) error {
	r := f.resets[id]
	r.Attempts++
	f.resets[id] = r
	return nil
}

func (f *fakeStore) SetPasswordAndVerify(_ context.Context, id, hash string) error {
	f.setPassword(id, hash)
	u := f.users[id]
	u.IsVerified = true
	f.users[id] = u
	f.users[u.Email] = u
	return nil
}

func (f *fakeStore) SetPassword(_ context.Context, id, hash string) error {
	f.setPassword(id, hash)
	return nil
}

func (f *fakeStore) setPassword(id, hash string) {
	u := f.users[id]
	u.Password = hash
	f.users[id] = u
	f.users[u.Email] = u
}

func (f *fakeStore) RevokeAllSessions(_ context.Context, userID string) error {
	f.sessions = map[string]bool{}
	return nil
}

func (f *fakeStore) RevokeOtherSessions(_ context.Context, userID, sessionID string) error {
	for k := range f.sessions {
		if k != sessionID {
			f.sessions[k] = false
		}
	}
	return nil
}

type fakeEmails struct{ sent []email.Message }

func (f *fakeEmails) Enqueue(_ context.Context, ms email.Message) error {
	f.sent = append(f.sent, ms)
	return nil
}
