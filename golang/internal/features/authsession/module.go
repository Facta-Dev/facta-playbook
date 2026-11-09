package authsession

import (
	"net/http"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/clockx"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store         Store
	Minter        TokenMinter
	Verifier      authx.Verifier
	Clock         clockx.Clock
	Pepper        string
	CookieName    string // "" disables the cookie fallback
	CookieSecure  bool   // Set-Cookie Secure flag (false only for dev)
	AccessTTL     time.Duration
	RefreshTTL    time.Duration
	MaxLoginRetry int
}

// Register mounts the slice's routes; protected routes sit behind RequireAuth
// and the admin auth-session endpoints additionally behind RequireRole(Admin)
// (reference /admin/auth-session; the admin edge is the rank gate).
func Register(mux *http.ServeMux, d Deps) {
	uc := New(d.Store, d.Minter, d.Clock, d.Pepper, d.AccessTTL, d.RefreshTTL, d.MaxLoginRetry)
	h := handlers{uc: uc, accessTTL: d.AccessTTL,
		cookieName: d.CookieName, cookieSecure: d.CookieSecure}
	auth := authx.RequireAuth(d.Verifier, d.CookieName)
	admin := func(next http.Handler) http.Handler {
		return auth(authx.RequireRole(rolex.Admin)(next))
	}
	mux.HandleFunc("POST /api/v1/auth/login", h.login)
	mux.HandleFunc("POST /api/v1/auth/refresh", h.refresh)
	mux.Handle("POST /api/v1/auth/logout", auth(http.HandlerFunc(h.logout)))
	mux.Handle("GET /api/v1/auth/me", auth(http.HandlerFunc(h.me)))
	mux.Handle("PUT /api/v1/auth/me", auth(http.HandlerFunc(h.updateMe)))
	// session-management views (reference /user/auth-session)
	mux.Handle("GET /api/v1/auth/sessions", auth(http.HandlerFunc(h.sessions)))
	mux.Handle("GET /api/v1/auth/sessions/current", auth(http.HandlerFunc(h.currentSession)))
	mux.Handle("POST /api/v1/auth/sessions/revoke-current", auth(http.HandlerFunc(h.revokeCurrent)))
	mux.Handle("POST /api/v1/auth/sessions/revoke-all", auth(http.HandlerFunc(h.revokeAll)))
	// admin auth-session (reference /admin/auth-session)
	mux.Handle("GET /api/v1/admin/auth-session/users/{id}/sessions", admin(http.HandlerFunc(h.adminSessions)))
	mux.Handle("POST /api/v1/admin/auth-session/users/{id}/revoke-all", admin(http.HandlerFunc(h.adminRevokeAll)))
	mux.Handle("GET /api/v1/admin/auth-session/anomalies", admin(http.HandlerFunc(h.anomalies)))
}
