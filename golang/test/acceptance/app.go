package acceptance

import (
	"net/http"
	"net/http/httptest"
	"time"

	"github.com/<scope>/<app>/internal/features/adminusers"
	"github.com/<scope>/<app>/internal/features/authoauth"
	"github.com/<scope>/<app>/internal/features/authpassword"
	"github.com/<scope>/<app>/internal/features/authsession"
	"github.com/<scope>/<app>/internal/features/authsignup"
	featrt "github.com/<scope>/<app>/internal/features/realtime"
	"github.com/<scope>/<app>/internal/features/widgets"
	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
	platrt "github.com/<scope>/<app>/internal/platform/realtime"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/test/testutil"
)

// TTLs of the flows under test; mirror the config defaults named in the
// features (24h verify, 15m access, 720h refresh, 30m reset).
const (
	verifyTTL  = 24 * time.Hour
	accessTTL  = 15 * time.Minute
	refreshTTL = 720 * time.Hour
	resetTTL   = 30 * time.Minute
)

// oauthFlags are the social-provider switches the toggle steps flip by
// rebuilding the app (the use case copies its config at Register time).
type oauthFlags struct {
	google, github bool
}

func newApp() *app { return newAppWith(oauthFlags{google: true, github: true}) }

// newAppWith assembles the real HTTP mux exactly like cmd/api/main.go — the
// same Register calls with in-memory store fakes, a recorded email enqueuer,
// a fixed clock and an in-process hub, then serves it on an httptest server.
// ponytail: healthz/readyz and the request-id/CORS chain are not assembled;
// no feature exercises them.
type app struct {
	mux    *http.ServeMux
	server *httptest.Server
	table  *userTable
	emails *testutil.RecordedEmails
	clock  *testutil.TestClock
	hub    platrt.EventBus
}

func newAppWith(flags oauthFlags) *app {
	clock := testutil.NewClock()
	table := newUserTable(clock)
	emails := testutil.NewEmails()
	verifier := testutil.NewVerifier()
	minter := testutil.NewMinter()
	hub := platrt.NewHub(16)

	mux := http.NewServeMux()
	authsignup.Register(mux, authsignup.Deps{
		Store: signupStore{table}, Emails: emails,
		Clock: clock, Pepper: testutil.TestPepper, VerifyTTL: verifyTTL,
	})
	authsession.Register(mux, authsession.Deps{
		Store: sessionStore{table}, Minter: minter, Verifier: verifier,
		Clock: clock, Pepper: testutil.TestPepper, CookieName: "",
		AccessTTL: accessTTL, RefreshTTL: refreshTTL, MaxLoginRetry: 5,
	})
	authpassword.Register(mux, authpassword.Deps{
		Store: passwordStore{table}, Emails: emails, Verifier: verifier,
		Clock: clock, Pepper: testutil.TestPepper, CookieName: "",
		ResetTTL: resetTTL, RateLimit: 3, RateWindow: 5 * time.Minute,
		LogOutAllOnReset: true,
	})
	adminusers.Register(mux, adminusers.Deps{
		Store: adminStore{table}, Verifier: verifier, CookieName: "",
		// The acceptance suite exercises the bootstrap route with a fixed
		// whitelist entry; see the upgrade-to-admin scenario.
		Whitelist: []string{"admin-boost@ex.com"},
	})
	authoauth.Register(mux, authoauth.Deps{
		Store:  oauthStore{table},
		Google: fakeGoogle{},
		GitHub: fakeGitHub{codes: map[string]oauthx.GitHubUser{
			"good-code": {ID: "gh-fresh", Login: "freshgh", Email: "fresh@ex.com"},
			"ada-code":  {ID: "gh-ada", Login: "ada", Name: "Ada L", Email: "ada@ex.com"},
		}},
		States: oauthx.NewLinkToken(testutil.TestJWTSecret),
		Minter: minter, Verifier: verifier, Clock: clock,
		GoogleClientID: "test-google-client-id", GoogleEnabled: flags.google,
		GitHubClientID: "test-github-client-id", GitHubClientSecret: "test-github-secret",
		GitHubCallbackURL: "http://localhost:8080/api/v1/auth/github/callback",
		GitHubEnabled:     flags.github,
		FrontendSuccess:   "http://localhost:5173/auth/success", Offline: false,
		CookieName: "", AccessTTL: accessTTL, RefreshTTL: refreshTTL,
	})
	widgets.Register(mux, widgets.Deps{
		Store: widgetStore{table}, Publisher: hub, Verifier: verifier, CookieName: "",
	})
	featrt.Register(mux, featrt.Deps{Verifier: verifier, Publisher: hub})

	// fallback: any unmatched path is the JSON 404 envelope (main.go parity).
	mux.HandleFunc("/", func(w http.ResponseWriter, _ *http.Request) {
		httpx.WriteErr(w, apperr.E(404, apperr.NotFound, "route not found"))
	})

	a := &app{mux: mux, table: table, emails: emails, clock: clock, hub: hub}
	a.server = httptest.NewServer(mux)
	return a
}
