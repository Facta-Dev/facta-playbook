package authoauth

import (
	"net/http"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store    Store
	Google   GoogleVerifier // oauthx
	GitHub   GitHubClient   // oauthx
	States   LinkState      // oauthx.LinkToken
	Minter   TokenMinter
	Verifier authx.Verifier // guards connect-url
	Clock    clockx.Clock

	GoogleClientID     string
	GoogleEnabled      bool
	GitHubClientID     string
	GitHubClientSecret string
	GitHubCallbackURL  string // <APP>_AUTH_GITHUB_CALLBACK_URL
	GitHubEnabled      bool
	FrontendSuccess    string // <APP>_AUTH_FRONTEND_SUCCESS
	Offline            bool   // INV-T7: refuse before any socket

	CookieName   string // "" disables the cookie fallback
	CookieSecure bool   // Set-Cookie Secure flag (false only for dev)
	AccessTTL    time.Duration
	RefreshTTL   time.Duration
}

// Register mounts the social-login routes. The google endpoint answers the
// standard JSON envelope (login shape); the github endpoints ALWAYS 302.
func Register(mux *http.ServeMux, d Deps) {
	uc := New(d.Store, d.Google, d.GitHub, d.States, d.Minter, d.Clock, Config{
		GoogleClientID:     d.GoogleClientID,
		GoogleEnabled:      d.GoogleEnabled,
		GitHubClientID:     d.GitHubClientID,
		GitHubClientSecret: d.GitHubClientSecret,
		GitHubCallbackURL:  d.GitHubCallbackURL,
		GitHubEnabled:      d.GitHubEnabled,
		FrontendSuccess:    d.FrontendSuccess,
		Offline:            d.Offline,
		AccessTTL:          d.AccessTTL,
		RefreshTTL:         d.RefreshTTL,
	})
	h := handlers{uc: uc, accessTTL: d.AccessTTL, cookieName: d.CookieName, cookieSecure: d.CookieSecure}
	mux.HandleFunc("POST /api/v1/auth/google", h.google)
	mux.HandleFunc("GET /api/v1/auth/github/login", h.githubLogin)
	mux.HandleFunc("GET /api/v1/auth/github/callback", h.githubCallback)
	mux.Handle("GET /api/v1/auth/github/connect-url",
		authx.RequireAuth(d.Verifier, d.CookieName)(http.HandlerFunc(h.githubConnect)))
}
