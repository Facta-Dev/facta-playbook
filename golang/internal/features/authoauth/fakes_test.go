package authoauth

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
)

// fakeStore is an in-memory Store keyed by id, email, google_id, github_id.
type fakeStore struct {
	users    map[string]User
	sessions map[string]bool // session id -> active
	created  []string        // provisioned user ids, in order
}

func newFakeStore(seed User) *fakeStore {
	f := &fakeStore{users: map[string]User{}, sessions: map[string]bool{}}
	f.put(seed)
	return f
}

func (f *fakeStore) put(u User) {
	f.users[u.ID] = u
	f.users[u.Email] = u
	if u.GoogleID != "" {
		f.users[u.GoogleID] = u
	}
	if u.GithubID != "" {
		f.users[u.GithubID] = u
	}
}

func (f *fakeStore) GetUserByGoogleID(_ context.Context, id string) (User, bool, error) {
	u, ok := f.users[id]
	return u, ok, nil
}

func (f *fakeStore) GetUserByGithubID(_ context.Context, id string) (User, bool, error) {
	u, ok := f.users[id]
	return u, ok, nil
}

func (f *fakeStore) GetUserByEmail(_ context.Context, email string) (User, bool, error) {
	u, ok := f.users[email]
	return u, ok, nil
}

func (f *fakeStore) GetUserByID(_ context.Context, id string) (User, bool, error) {
	u, ok := f.users[id]
	return u, ok, nil
}

func (f *fakeStore) CreateUser(_ context.Context, u User) error {
	f.put(u)
	f.created = append(f.created, u.ID)
	return nil
}

func (f *fakeStore) LinkGoogle(_ context.Context, userID, providerID, picture string) error {
	u := f.users[userID]
	u.GoogleID, u.IsVerified = providerID, true
	if u.Picture == "" {
		u.Picture = picture
	}
	f.put(u)
	return nil
}

func (f *fakeStore) LinkGithub(_ context.Context, userID, providerID, picture string) error {
	u := f.users[userID]
	u.GithubID, u.IsVerified = providerID, true
	if u.Picture == "" {
		u.Picture = picture
	}
	f.put(u)
	return nil
}

func (f *fakeStore) GithubOnly(_ context.Context, userID, githubID string) error {
	u := f.users[userID]
	u.GithubID = githubID
	f.put(u)
	return nil
}

func (f *fakeStore) SaveGithubLogin(_ context.Context, userID, email, picture string) error {
	u := f.users[userID]
	if u.Email != email {
		delete(f.users, u.Email)
		u.Email = email
	}
	u.IsVerified = true
	if u.Picture == "" {
		u.Picture = picture
	}
	f.put(u)
	return nil
}

func (f *fakeStore) CreateSession(_ context.Context, s SessionRow) error {
	f.sessions[s.ID] = true
	return nil
}

// fakeVerifier always returns the configured identity (or an error).
type fakeVerifier struct {
	id  oauthx.GoogleIdentity
	err error
}

func (v fakeVerifier) Verify(_ context.Context, _, _ string) (oauthx.GoogleIdentity, error) {
	return v.id, v.err
}

// fakeGitHub plays the client against pre-cooked exchanges.
type fakeGitHub struct {
	user oauthx.GitHubUser
	err  error
}

func (g fakeGitHub) Authorize(clientID, _redirectURI, state string) string {
	return "https://github.example/login?client_id=" + clientID + "&state=" + state
}

func (g fakeGitHub) Exchange(_ context.Context, _, _, _, _ string) (oauthx.GitHubUser, error) {
	return g.user, g.err
}

// fakeMinter hands out a deterministic access token.
type fakeMinter struct{ minted authx.Identity }

func (m *fakeMinter) Mint(id authx.Identity, _ time.Duration) (string, time.Time, error) {
	m.minted = id
	return "access-" + id.UserID, time.Now().Add(15 * time.Minute), nil
}
