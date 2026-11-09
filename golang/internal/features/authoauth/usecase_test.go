package authoauth

import (
	"errors"
	"strings"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/platform/oauthx"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// states is the real codec over a test secret (roundtrips the flow).
var states = oauthx.NewLinkToken("0123456789abcdef0123456789abcdef")

// newUC wires the use-case; everything on, fixed clock.
func newUC(store Store, g GoogleVerifier, gh GitHubClient, now time.Time) (*UseCase, *fakeMinter) {
	m := &fakeMinter{}
	uc := New(store, g, gh, states, m, clockx.Fixed(now), Config{
		GoogleClientID:     "g-client",
		GoogleEnabled:      true,
		GitHubClientID:     "gh-client",
		GitHubClientSecret: "gh-secret",
		GitHubCallbackURL:  "http://cb.example/callback",
		GitHubEnabled:      true,
		FrontendSuccess:    "http://localhost:5173/auth/success",
		AccessTTL:          15 * time.Minute,
		RefreshTTL:         720 * time.Hour,
	})
	return uc, m
}

func TestGoogleProvisionsAndLogsIn(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(User{})
	uc, minter := newUC(store, fakeVerifier{id: oauthx.GoogleIdentity{
		Sub: "g-1", Email: "a@ex.com", GivenName: "Ada", FamilyName: "Lovelace", Picture: "gpic",
	}}, fakeGitHub{}, now)

	res, err := uc.Google(t.Context(), "cred", "10.0.0.1", "ua")
	if err != nil {
		t.Fatalf("google: %v", err)
	}
	u := store.users["a@ex.com"]
	if u.ID == "" || len(store.created) != 1 {
		t.Fatalf("user not provisioned: %+v", store.users)
	}
	if !u.IsVerified || u.UserType != "COMPANY" || u.GoogleID != "g-1" || u.Picture != "gpic" {
		t.Fatalf("bad provisioned user: %+v", u)
	}
	if !store.sessions[minter.minted.JTI] {
		t.Fatal("session row missing")
	}
	if res.AccessToken == "" || res.RefreshToken == "" || res.ExpiresAt.Before(now) {
		t.Fatalf("bad result: %+v", res)
	}
}

func TestGoogleLinksByEmailThenConflicts(t *testing.T) {
	t.Parallel()
	store := newFakeStore(User{ID: "u-1", Email: "a@ex.com", FirstName: "Ada", Role: "USER",
		UserType: "COMPANY", Language: "en", IsActive: true, IsVerified: false})
	uc, _ := newUC(store, fakeVerifier{id: oauthx.GoogleIdentity{
		Sub: "g-1", Email: "a@ex.com", Picture: "gpic",
	}}, fakeGitHub{}, time.Unix(1700, 0))

	res, err := uc.Google(t.Context(), "cred", "", "")
	if err != nil {
		t.Fatalf("link: %v", err)
	}
	u := store.users["u-1"]
	if u.GoogleID != "g-1" || !u.IsVerified || u.Picture != "gpic" {
		t.Fatalf("link not applied: %+v", u)
	}
	if res.RefreshToken == "" {
		t.Fatal("session not issued")
	}

	// A different Google account claiming the same email is a hard stop:
	// a fresh store whose email-matched user already carries another g id.
	conflict := newFakeStore(User{ID: "u-1", Email: "a@ex.com", Role: "USER", Language: "en",
		GoogleID: "g-other", IsActive: true, IsVerified: true})
	uc, _ = newUC(conflict, fakeVerifier{id: oauthx.GoogleIdentity{Sub: "g-1", Email: "a@ex.com"}},
		fakeGitHub{}, time.Unix(1700, 0))
	if _, err := uc.Google(t.Context(), "cred", "", ""); !errors.Is(err, ErrGoogleEmailLinked) {
		t.Fatalf("want ErrGoogleEmailLinked, got %v", err)
	}
}

func TestGoogleGatesAndRefusals(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)

	// Blocked / inactive users found by google id are refused.
	for _, tc := range []struct {
		user User
		want error
	}{{User{ID: "u-1", Email: "a@ex.com", Role: "USER", Language: "en",
		GoogleID: "g-1", IsActive: true, IsBlocked: true, IsVerified: true}, ErrAccountBlocked},
		{User{ID: "u-2", Email: "b@ex.com", Role: "USER", Language: "en",
			GoogleID: "g-2", IsActive: false, IsVerified: true}, ErrAccountInactive}} {
		store := newFakeStore(tc.user)
		uc, _ := newUC(store, fakeVerifier{id: oauthx.GoogleIdentity{Sub: tc.user.GoogleID, Email: tc.user.Email}},
			fakeGitHub{}, now)
		if _, err := uc.Google(t.Context(), "cred", "", ""); !errors.Is(err, tc.want) {
			t.Fatalf("want %v, got %v", tc.want, err)
		}
	}

	// Disabled (or offline) refuses before touching the verifier.
	store := newFakeStore(User{})
	uc, _ := newUC(store, fakeVerifier{}, fakeGitHub{}, now)
	uc.googleOn = false
	if _, err := uc.Google(t.Context(), "cred", "", ""); !errors.Is(err, ErrGoogleNotConfigured) {
		t.Fatalf("want disabled refusal, got %v", err)
	}
	uc.offline = true
	uc.google = fakeVerifier{id: oauthx.GoogleIdentity{Sub: "g-1", Email: "a@ex.com"}}
	if _, err := uc.Google(t.Context(), "cred", "", ""); !errors.Is(err, ErrGoogleNotConfigured) {
		t.Fatalf("want offline refusal, got %v", err)
	}

	// A failed verification is the generic invalid-token 401.
	uc.offline = false
	uc.googleOn = true
	uc.google = fakeVerifier{err: errors.New("bad sig")}
	if _, err := uc.Google(t.Context(), "cred", "", ""); !errors.Is(err, ErrInvalidGoogleToken) {
		t.Fatalf("want ErrInvalidGoogleToken, got %v", err)
	}
}

func TestGithubProvisionsAndRefreshes(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	gh := fakeGitHub{user: oauthx.GitHubUser{ID: "1001", Login: "ada", Name: "Ada Lovelace",
		Email: "ada@ex.com", AvatarURL: "av"}}

	store := newFakeStore(User{})
	uc, minter := newUC(store, fakeVerifier{}, gh, now)
	res, err := uc.GithubLogin(t.Context(), "code", "10.0.0.2", "ua")
	if err != nil {
		t.Fatalf("github login: %v", err)
	}
	u := store.users["ada@ex.com"]
	if u.ID == "" || u.FirstName != "Ada" || u.LastName != "Lovelace" || u.GithubID != "1001" ||
		!u.IsVerified || u.Picture != "av" {
		t.Fatalf("bad github provision: %+v", u)
	}
	if !store.sessions[minter.minted.JTI] || res.AccessToken == "" {
		t.Fatal("session not issued")
	}

	// A second ride upgrades a noreply email and fills the empty picture.
	store = newFakeStore(User{ID: "u-9", Email: "1001@users.noreply.github.com", Role: "USER",
		Language: "en", GithubID: "1001", IsActive: true, IsVerified: false})
	uc, _ = newUC(store, fakeVerifier{}, gh, now)
	if _, err := uc.GithubLogin(t.Context(), "code", "", ""); err != nil {
		t.Fatalf("refresh login: %v", err)
	}
	u = store.users["u-9"]
	if u.Email != "ada@ex.com" || !u.IsVerified || u.Picture != "av" {
		t.Fatalf("identity not refreshed: %+v", u)
	}
}

func TestGithubLinkFlow(t *testing.T) {
	t.Parallel()
	store := newFakeStore(User{ID: "u-1", Email: "a@ex.com", Role: "USER", Language: "en",
		IsActive: true, IsVerified: true})
	uc, _ := newUC(store, fakeVerifier{}, fakeGitHub{user: oauthx.GitHubUser{ID: "777"}}, time.Unix(1700, 0))

	// connect-url hands the frontend an authorize link whose state is link:<jwt>.
	authorized, err := uc.GithubConnectURL("u-1")
	if err != nil {
		t.Fatalf("connect-url: %v", err)
	}
	const stateMark = "&state=" + LinkStatePrefix
	if !strings.Contains(authorized, "client_id=gh-client") || !strings.Contains(authorized, stateMark) {
		t.Fatalf("authorize url missing pieces: %s", authorized)
	}
	// The callback passes the state minus its "link:" prefix.
	state := authorized[strings.Index(authorized, stateMark)+len(stateMark):]
	if err := uc.GithubLink(t.Context(), state, "code"); err != nil {
		t.Fatalf("link: %v", err)
	}
	if store.users["u-1"].GithubID != "777" {
		t.Fatalf("github id not attached: %+v", store.users["u-1"])
	}

	// The same GitHub id on another user is a conflict.
	store.put(User{ID: "u-2", Email: "b@ex.com", Role: "USER", Language: "en", IsActive: true})
	other, _ := uc.GithubConnectURL("u-2")
	otherState := other[strings.Index(other, stateMark)+len(stateMark):]
	if err := uc.GithubLink(t.Context(), otherState, "code"); !errors.Is(err, ErrGithubAlreadyLinked) {
		t.Fatalf("want ErrGithubAlreadyLinked, got %v", err)
	}

	// A tampered state is the generic invalid-link refusal.
	if err := uc.GithubLink(t.Context(), "nonsense", "code"); !errors.Is(err, ErrInvalidLinkState) {
		t.Fatalf("want ErrInvalidLinkState, got %v", err)
	}
}

func TestGithubRefusalsAndLoginURL(t *testing.T) {
	t.Parallel()
	store := newFakeStore(User{})
	uc, _ := newUC(store, fakeVerifier{}, fakeGitHub{}, time.Unix(1700, 0))
	authorized, err := uc.GithubLoginURL("s1")
	if err != nil {
		t.Fatalf("login url: %v", err)
	}
	if !strings.Contains(authorized, "state=s1") {
		t.Fatalf("state passthrough missing: %s", authorized)
	}

	// Missing client secret still gates the login callback.
	uc.githubSecret = ""
	if _, err := uc.GithubLogin(t.Context(), "code", "", ""); !errors.Is(err, ErrGithubNotConfigured) {
		t.Fatalf("want ErrGithubNotConfigured, got %v", err)
	}
	uc.githubSecret = "gh-secret"

	uc.githubOn = false
	if _, err := uc.GithubLoginURL(""); !errors.Is(err, ErrGithubNotConfigured) {
		t.Fatalf("want disabled refusal, got %v", err)
	}
	uc.offline = true
	if _, err := uc.GithubLoginURL(""); !errors.Is(err, ErrGithubNotConfigured) {
		t.Fatalf("want offline refusal, got %v", err)
	}
}

func TestGithubNamesDefaults(t *testing.T) {
	t.Parallel()
	first, last := githubNames(oauthx.GitHubUser{Login: "ada"})
	if first != "ada" || last != "GitHub" {
		t.Fatalf("name fallback broken: %q %q", first, last)
	}
	first, last = googleNames(oauthx.GoogleIdentity{})
	if first != "User" || last != "Google" {
		t.Fatalf("google name fallback broken: %q %q", first, last)
	}
}
