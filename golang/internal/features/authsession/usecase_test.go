package authsession

import (
	"errors"
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/shared/apperr"
)

func TestLoginSocialAccountRejectsWrongPassword(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	cases := []struct {
		name      string
		google    bool
		github    bool
		wantSubst string
	}{
		{"google-only", true, false, "created with Google"},
		{"github-only", false, true, "created with GitHub"},
		{"both", true, true, "created with Google and GitHub"},
	}
	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			store := newFakeStore(now)
			uc := newUC(store, &stubMinter{}, now)
			// The reference fires the guidance only when the account really
			// carries the provider id, so seed the row's linked flags.
			u := store.users["a@ex.com"]
			u.GoogleLinked, u.GithubLinked = tc.google, tc.github
			store.users["a@ex.com"] = u
			// The retry increment still happens before the guidance (parity
			// with the reference: brute-forcing a social account still locks).
			_, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "wrong"})
			var ae *apperr.AppError
			if !errors.As(err, &ae) || ae.Code != apperr.AuthSignedUpWithSocial {
				t.Fatalf("want AUTH_SIGNED_UP_WITH_SOCIAL, got %v", err)
			}
			if store.users["a@ex.com"].RetryCount != 1 {
				t.Fatalf("retry not counted: %d", store.users["a@ex.com"].RetryCount)
			}
			if got := socialGuidance(tc.google, tc.github); !strings.Contains(got, tc.wantSubst) {
				t.Fatalf("guidance %q must mention %q", got, tc.wantSubst)
			}
		})
	}
	// A set password still wins: the guard fires only on a WRONG password.
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	u := store.users["a@ex.com"]
	u.GoogleLinked, u.GithubLinked = true, true
	u.PasswordHash = hashForTest(t, "correct-horse", "pepper")
	store.users["a@ex.com"] = u
	if _, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "correct-horse"}); err != nil {
		t.Fatalf("correct password on linked account: %v", err)
	}
}

func TestLoginGates(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	cases := []struct {
		name  string
		mut   func(*fakeStore)
		token string
	}{
		{"blocked", func(s *fakeStore) { s.mut("id-a@ex.com", setBlock(true)) }, apperr.AuthAccountBlocked},
		{"inactive", func(s *fakeStore) { s.mut("id-a@ex.com", setActive(false)) }, apperr.AuthAccountInactive},
		{"unverified", func(s *fakeStore) { s.mut("id-a@ex.com", setVerified(false)) }, apperr.AuthEmailNotVerified},
	}
	for _, tc := range cases {
		tc := tc
		// Correct password: only then the state gate fires.
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			store := newFakeStore(now)
			tc.mut(store)
			seedHash(t, store)
			uc := newUC(store, &stubMinter{}, now)
			_, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: correctPass})
			var ae *apperr.AppError
			if !errors.As(err, &ae) || ae.Code != tc.token {
				t.Fatalf("want %s, got %v", tc.token, err)
			}
		})
		// Wrong password: the state must NOT leak — one constant 401, and the
		// retry lock still counts the attempt.
		t.Run(tc.name+"-wrong-password", func(t *testing.T) {
			t.Parallel()
			store := newFakeStore(now)
			tc.mut(store)
			seedHash(t, store)
			uc := newUC(store, &stubMinter{}, now)
			_, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "not-it"})
			var ae *apperr.AppError
			if !errors.As(err, &ae) || ae.Code != apperr.AuthInvalidCredentials {
				t.Fatalf("want constant AUTH_INVALID_CREDENTIALS, got %v", err)
			}
			if store.users["a@ex.com"].RetryCount != 1 {
				t.Fatalf("retry not counted: %d", store.users["a@ex.com"].RetryCount)
			}
		})
	}
}

func TestLoginWrongPasswordLocksAtMax(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	// The stub row stores no real hash: CheckPassword always fails.
	for i := 1; i <= 5; i++ {
		_, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "wrong"})
		if err == nil {
			t.Fatal("want failure")
		}
		if i < 5 && store.users["a@ex.com"].RetryCount != i {
			t.Fatalf("retry %d, want %d", store.users["a@ex.com"].RetryCount, i)
		}
	}
	if !store.users["a@ex.com"].IsBlocked {
		t.Fatal("not blocked at max retries")
	}
}

func TestLoginFullRoundTrip(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	minter := &stubMinter{}
	uc := newUC(store, minter, now)
	u := store.users["a@ex.com"]
	u.PasswordHash = hashForTest(t, "correct-horse", "pepper")
	store.users["a@ex.com"] = u

	res, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "correct-horse", IPAddress: "1.2.3.4"})
	if err != nil {
		t.Fatalf("login: %v", err)
	}
	if res.AccessToken == "" || res.RefreshToken == "" {
		t.Fatal("missing tokens")
	}
	s := store.sessions[res.RefreshToken]
	if s.ID != res.AccessToken[len("access-token."):] {
		t.Fatalf("jti != session id: %q vs %q", res.AccessToken, s.ID)
	}
	if s.IPAddress != "1.2.3.4" {
		t.Fatalf("bad session metadata: %+v", s)
	}
	if minter.calls != 1 {
		t.Fatalf("minter calls: %d", minter.calls)
	}
}

func TestRefreshRotatesInPlace(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	u := store.users["a@ex.com"]
	u.PasswordHash = hashForTest(t, "correct-horse", "pepper")
	store.users["a@ex.com"] = u

	res, err := uc.Login(t.Context(), LoginInput{Email: "a@ex.com", Password: "correct-horse"})
	if err != nil {
		t.Fatalf("login: %v", err)
	}
	origID := res.AccessToken[len("access-token."):]
	origRow := store.sessions[res.RefreshToken]
	origExpiry := origRow.ExpiresAt

	fresh, err := uc.Refresh(t.Context(), res.RefreshToken, "5.6.7.8", "ua")
	if err != nil {
		t.Fatalf("refresh: %v", err)
	}
	if fresh.RefreshToken == res.RefreshToken {
		t.Fatal("token not rotated")
	}
	// The row survives in place: same session id (stable jti == session id),
	// same expires_at — rotation never slides the deadline.
	s := store.sessions[fresh.RefreshToken]
	if s.ID != origID {
		t.Fatalf("session id changed on rotation: %q vs %q", s.ID, origID)
	}
	if !s.ExpiresAt.Equal(origExpiry) {
		t.Fatalf("expires_at moved on rotation: %v vs %v", s.ExpiresAt, origExpiry)
	}
	if s.IPAddress != "5.6.7.8" {
		t.Fatalf("ip not updated: %q", s.IPAddress)
	}
	// The old token is gone; reusing it must fail.
	if _, ok := store.sessions[res.RefreshToken]; ok {
		t.Fatal("old refresh token still present")
	}
	if _, err := uc.Refresh(t.Context(), res.RefreshToken, "", ""); !errors.Is(err, ErrInvalidRefresh) {
		t.Fatalf("replayed old token: %v", err)
	}
	// But the fresh token rotates again (jti unchanged).
	fresh2, err := uc.Refresh(t.Context(), fresh.RefreshToken, "", "")
	if err != nil || fresh2.RefreshToken == fresh.RefreshToken {
		t.Fatalf("second rotation: %v %v", fresh2, err)
	}
}

func TestRefreshRejectsUnknown(t *testing.T) {
	t.Parallel()
	store := newFakeStore(time.Unix(1700, 0))
	uc := newUC(store, &stubMinter{}, time.Unix(1700, 0))
	if _, err := uc.Refresh(t.Context(), "nope", "", ""); !errors.Is(err, ErrInvalidRefresh) {
		t.Fatalf("unknown token: %v", err)
	}
}

func TestLogoutRevokesOnlyTheGivenToken(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	exp := now.Add(time.Hour)
	store.sessions["tok-1"] = SessionRow{ID: "s1", RefreshToken: "tok-1", UserID: "id-a@ex.com", ExpiresAt: exp, IsActive: true}
	store.sessions["tok-2"] = SessionRow{ID: "s2", RefreshToken: "tok-2", UserID: "id-a@ex.com", ExpiresAt: exp, IsActive: true}

	if err := uc.Logout(t.Context(), "id-a@ex.com", "tok-1"); err != nil {
		t.Fatalf("logout: %v", err)
	}
	if store.sessions["tok-1"].IsActive {
		t.Fatal("tok-1 still active")
	}
	if !store.sessions["tok-2"].IsActive {
		t.Fatal("tok-2 must stay active")
	}
	if err := uc.Logout(t.Context(), "id-a@ex.com", "tok-1"); err != nil {
		t.Fatalf("repeat logout: %v", err)
	}
}

// addSession plants a session row directly (token is the fake's map key).
func addSession(store *fakeStore, s SessionRow) { store.sessions[s.RefreshToken] = s }

func TestSessionsListFiltersActive(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	exp := now.Add(time.Hour)
	addSession(store, SessionRow{ID: "s1", RefreshToken: "tok-1", UserID: "id-a@ex.com", ExpiresAt: exp, LastActivity: now, IsActive: true})
	addSession(store, SessionRow{ID: "s2", RefreshToken: "tok-2", UserID: "id-a@ex.com", ExpiresAt: exp, LastActivity: now.Add(-time.Minute), IsActive: true})
	addSession(store, SessionRow{ID: "s3", RefreshToken: "tok-3", UserID: "id-a@ex.com", ExpiresAt: exp, LastActivity: now.Add(-2 * time.Minute), IsActive: false})

	active, err := uc.Sessions(t.Context(), "id-a@ex.com", true)
	if err != nil {
		t.Fatalf("sessions: %v", err)
	}
	if len(active) != 2 || active[0].ID != "s1" {
		t.Fatalf("active list: %+v", active)
	}
	all, err := uc.Sessions(t.Context(), "id-a@ex.com", false)
	if err != nil || len(all) != 3 {
		t.Fatalf("all list: %d %v", len(all), err)
	}
}

func TestCurrentSessionPicksLatestActivity(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	exp := now.Add(time.Hour)
	addSession(store, SessionRow{ID: "s1", RefreshToken: "tok-1", UserID: "id-a@ex.com", ExpiresAt: exp, LastActivity: now.Add(-time.Minute), IsActive: true})
	addSession(store, SessionRow{ID: "s2", RefreshToken: "tok-2", UserID: "id-a@ex.com", ExpiresAt: exp, LastActivity: now, IsActive: true})

	row, ok, err := uc.CurrentSession(t.Context(), "id-a@ex.com")
	if err != nil || !ok {
		t.Fatalf("current: %v ok=%v", err, ok)
	}
	if row.ID != "s2" {
		t.Fatalf("picked %q, want latest-activity s2", row.ID)
	}
	if _, ok, _ := uc.CurrentSession(t.Context(), "id-nobody@ex.com"); ok {
		t.Fatal("no session should read false for an unknown user")
	}
}

func TestRevokeCurrentClaimsOnlyOwnActiveSession(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	addSession(store, SessionRow{ID: "s1", RefreshToken: "tok-1", UserID: "id-a@ex.com", ExpiresAt: now.Add(time.Hour), IsActive: true})
	addSession(store, SessionRow{ID: "s9", RefreshToken: "tok-9", UserID: "id-b@ex.com", ExpiresAt: now.Add(time.Hour), IsActive: true})

	revoked, err := uc.RevokeCurrent(t.Context(), "id-a@ex.com", "s1")
	if err != nil || !revoked {
		t.Fatalf("revoke: %v revoked=%v", err, revoked)
	}
	if store.sessions["tok-1"].IsActive {
		t.Fatal("s1 still active after revoke")
	}
	if revoked, _ := uc.RevokeCurrent(t.Context(), "id-a@ex.com", "s1"); revoked {
		t.Fatal("second claim of s1 must lose the CAS")
	}
	if revoked, _ := uc.RevokeCurrent(t.Context(), "id-a@ex.com", "s9"); revoked {
		t.Fatal("another user's session must not be revocable")
	}
	if revoked, _ := uc.RevokeCurrent(t.Context(), "id-a@ex.com", ""); revoked {
		t.Fatal("empty session id must read as not-revoked")
	}
}

func TestRevokeAllClaimsOnlyActive(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	exp := now.Add(time.Hour)
	addSession(store, SessionRow{ID: "s1", RefreshToken: "tok-1", UserID: "id-a@ex.com", ExpiresAt: exp, IsActive: true})
	addSession(store, SessionRow{ID: "s2", RefreshToken: "tok-2", UserID: "id-a@ex.com", ExpiresAt: exp, IsActive: true})
	addSession(store, SessionRow{ID: "s3", RefreshToken: "tok-3", UserID: "id-a@ex.com", ExpiresAt: exp, IsActive: false})
	addSession(store, SessionRow{ID: "s9", RefreshToken: "tok-9", UserID: "id-b@ex.com", ExpiresAt: exp, IsActive: true})

	n, err := uc.RevokeAll(t.Context(), "id-a@ex.com")
	if err != nil || n != 2 {
		t.Fatalf("revoke-all: %v n=%d", err, n)
	}
	if !store.sessions["tok-9"].IsActive {
		t.Fatal("other user's session must survive")
	}
	if n, _ := uc.RevokeAll(t.Context(), "id-a@ex.com"); n != 0 {
		t.Fatalf("second revoke-all must be a no-op, got %d", n)
	}
}

func TestAnomaliesReport(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store := newFakeStore(now)
	uc := newUC(store, &stubMinter{}, now)
	// expired-but-active
	addSession(store, SessionRow{ID: "s0", RefreshToken: "tok-0", UserID: "id-a@ex.com", ExpiresAt: now.Add(-time.Hour), IsActive: true})
	// user b: four active sessions over four distinct IPs inside the window
	for i, ip := range []string{"1.0.0.1", "1.0.0.2", "1.0.0.3", "1.0.0.4"} {
		addSession(store, SessionRow{ID: fmt.Sprintf("sb%d", i), RefreshToken: fmt.Sprintf("tok-b%d", i),
			UserID: "id-b@ex.com", IPAddress: ip, ExpiresAt: now.Add(time.Hour), LastActivity: now, IsActive: true})
	}
	// inside the window but well-behaved: 1 IP must not anomaly
	addSession(store, SessionRow{ID: "sa", RefreshToken: "tok-a", UserID: "id-a@ex.com", IPAddress: "2.0.0.1", ExpiresAt: now.Add(time.Hour), LastActivity: now, IsActive: true})

	rows, err := uc.Anomalies(t.Context(), time.Hour)
	if err != nil {
		t.Fatalf("anomalies: %v", err)
	}
	if len(rows) != 2 || rows[0].Type != "expired_but_active" || rows[0].Count != 1 {
		t.Fatalf("anomalies: %+v", rows)
	}
	if rows[1].Type != "multi_ip_sessions" || rows[1].Count != 1 {
		t.Fatalf("multi-ip anomaly: %+v", rows[1])
	}
	if ids, _ := rows[1].Details["user_ids"].([]string); len(ids) != 1 || ids[0] != "id-b@ex.com" {
		t.Fatalf("multi-ip details: %+v", rows[1].Details)
	}
}

func TestAnomaliesRejectsOutOfRangeWindow(t *testing.T) {
	t.Parallel()
	store := newFakeStore(time.Unix(1700, 0))
	uc := newUC(store, &stubMinter{}, time.Unix(1700, 0))
	for _, window := range []time.Duration{0, -time.Hour, 31 * 24 * time.Hour} {
		_, err := uc.Anomalies(t.Context(), window)
		var ae *apperr.AppError
		if !errors.As(err, &ae) || ae.Code != apperr.Validation {
			t.Fatalf("window %v: want validation error, got %v", window, err)
		}
	}
}
