package authx

import (
	"net/http"
	"strings"

	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// Middleware wraps a handler.
type Middleware func(http.Handler) http.Handler

// bearerToken extracts the raw token: `Authorization: Bearer` header first,
// then the configured cookie when cookieName != "". WS/SSE handlers pass
// cookieName="" and read the `token` query param themselves.
// In cookie mode the name is "<app>_access_token" (rename-substituted).
func bearerToken(r *http.Request, cookieName string) (raw string, viaCookie bool) {
	const prefix = "Bearer "
	if h := r.Header.Get("Authorization"); strings.HasPrefix(h, prefix) {
		return strings.TrimSpace(strings.TrimPrefix(h, prefix)), false
	}
	if cookieName != "" {
		if c, err := r.Cookie(cookieName); err == nil {
			return c.Value, true
		}
	}
	return "", false
}

// RequireAuth verifies the token and stores the Identity on the request
// context; otherwise it answers 401 with the error envelope. Protected-mount
// only: /healthz and /readyz are mounted outside it.
func RequireAuth(v Verifier, cookieName string) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			raw, _ := bearerToken(r, cookieName)
			if raw == "" {
				httpx.WriteErr(w, apperr.E(apperr.CodeUnauthorized, apperr.AuthUnauthorized, "authentication required"))
				return
			}
			id, err := v.Parse(raw)
			if err != nil {
				httpx.WriteErr(w, err)
				return
			}
			next.ServeHTTP(w, r.WithContext(WithIdentity(r.Context(), id)))
		})
	}
}

// SetAuthCookie issues the cookie-mode access-token cookie: HttpOnly,
// SameSite=Strict, Path=/ and Secure everywhere except the local dev env
// (see <APP>_AUTH_COOKIE in .env.example). Bearer mode is always accepted.
func SetAuthCookie(w http.ResponseWriter, name, token string, secure bool) {
	http.SetCookie(w, &http.Cookie{Name: name, Value: token, Path: "/",
		HttpOnly: true, Secure: secure, SameSite: http.SameSiteStrictMode})
}

// ClearAuthCookie expires the cookie-mode access-token cookie on logout.
func ClearAuthCookie(w http.ResponseWriter, name string, secure bool) {
	http.SetCookie(w, &http.Cookie{Name: name, Value: "", Path: "/", MaxAge: -1,
		HttpOnly: true, Secure: secure, SameSite: http.SameSiteStrictMode})
}

// RequireRole authorizes by rank (rolex.AtLeast) on the identity stored by
// RequireAuth; otherwise 403 AUTH_FORBIDDEN envelope.
func RequireRole(min rolex.Role) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			id, ok := IdentityFrom(r.Context())
			if !ok || !rolex.AtLeast(id.Role, min) {
				httpx.WriteErr(w, apperr.E(apperr.CodeForbidden, apperr.AuthForbidden, "insufficient role"))
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}
