package adminusers

import (
	"net/http"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// Deps carries everything Register needs; main.go is the only caller.
type Deps struct {
	Store      Store
	Verifier   authx.Verifier
	CookieName string   // forwarded to RequireAuth
	Whitelist  []string // <APP>_ALLOWED_ADMIN_EMAILS for the bootstrap upgrade
}

// Register mounts the slice's routes: RequireAuth verifies the bearer/cookie
// token and stores the Identity; RequireRole(admin) then gates by rank —
// an admin request is 401 when unauthenticated and 403 when not admin.
func Register(mux *http.ServeMux, d Deps) {
	h := handlers{uc: New(d.Store, d.Whitelist)}
	auth := authx.RequireAuth(d.Verifier, d.CookieName)
	admin := func(next http.Handler) http.Handler {
		return auth(authx.RequireRole(rolex.Admin)(next))
	}
	mux.Handle("GET /api/v1/admin/users", admin(http.HandlerFunc(h.list)))
	mux.Handle("GET /api/v1/admin/users/{id}", admin(http.HandlerFunc(h.get)))
	mux.Handle("PUT /api/v1/admin/users/{id}", admin(http.HandlerFunc(h.update)))
	// Bootstrap whitelist upgrade: unauthenticated by design — the
	// <APP>_ALLOWED_ADMIN_EMAILS whitelist is the only gate (reference
	// upgrade_to_admin parity).
	mux.Handle("POST /api/v1/auth/upgrade-to-admin", http.HandlerFunc(h.upgradeToAdmin))
}
