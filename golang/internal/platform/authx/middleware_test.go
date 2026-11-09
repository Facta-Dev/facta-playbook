package authx

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

type stubVerifier struct {
	id  Identity
	err error
}

func (s stubVerifier) Parse(raw string) (Identity, error) { return s.id, s.err }

type ctxKeyRecorder struct {
	got  Identity
	have bool
}

func (rec *ctxKeyRecorder) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	rec.got, rec.have = IdentityFrom(r.Context())
	w.WriteHeader(http.StatusTeapot)
}

func bearerRequest(raw string) *http.Request {
	r := httptest.NewRequest(http.MethodGet, "/anything", nil)
	r.Header.Set("Authorization", "Bearer "+raw)
	return r
}

func TestRequireAuthHappy(t *testing.T) {
	t.Parallel()
	rec := &ctxKeyRecorder{}
	want := Identity{UserID: "u-9", Email: "u-9@example.com", Role: rolex.User, JTI: "j-1"}
	chain := RequireAuth(stubVerifier{id: want}, "")(rec)
	res := httptest.NewRecorder()
	chain.ServeHTTP(res, bearerRequest("raw-token"))
	if res.Code != http.StatusTeapot {
		t.Fatalf("next handler not reached: %d", res.Code)
	}
	if !rec.have || rec.got != want {
		t.Fatalf("identity not stored: have=%v got=%+v", rec.have, rec.got)
	}
}

func TestRequireAuthCookieFallback(t *testing.T) {
	t.Parallel()
	rec := &ctxKeyRecorder{}
	want := Identity{UserID: "u-9", Email: "u-9@example.com", Role: rolex.User, JTI: "j-1"}
	chain := RequireAuth(stubVerifier{id: want}, "test_access_token")(rec)
	r := httptest.NewRequest(http.MethodGet, "/anything", nil)
	r.AddCookie(&http.Cookie{Name: "test_access_token", Value: "raw-cookie-token"})
	res := httptest.NewRecorder()
	chain.ServeHTTP(res, r)
	if res.Code != http.StatusTeapot || !rec.have || rec.got != want {
		t.Fatalf("cookie path failed: code=%d have=%v", res.Code, rec.have)
	}
}

func TestRequireAuth401(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name    string
		setAuth bool
		verErr  bool
		code    string
	}{
		{name: "missing token", setAuth: false, code: "AUTH_UNAUTHORIZED"},
		{name: "bad token", setAuth: true, verErr: true, code: "AUTH_INVALID_TOKEN"},
	}
	for _, tc := range tests {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			t.Parallel()
			v := stubVerifier{}
			if tc.verErr {
				v.err = apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "invalid or expired token")
			}
			h := RequireAuth(v, "")(http.NotFoundHandler())
			r := bearerRequest("raw")
			if !tc.setAuth {
				r.Header.Del("Authorization")
			}
			res := httptest.NewRecorder()
			h.ServeHTTP(res, r)
			if res.Code != http.StatusUnauthorized {
				t.Fatalf("want 401, got %d", res.Code)
			}
			var env struct {
				Code string `json:"code"`
			}
			if err := json.Unmarshal(res.Body.Bytes(), &env); err != nil {
				t.Fatalf("envelope: %v", err)
			}
			if env.Code != tc.code {
				t.Fatalf("want code %s, got %s", tc.code, env.Code)
			}
		})
	}
}

func TestRequireRole403(t *testing.T) {
	t.Parallel()
	v := stubVerifier{id: Identity{UserID: "u-9", Role: rolex.User}}
	h := RequireRole(rolex.Admin)(http.NotFoundHandler())
	res := httptest.NewRecorder()
	RequireAuth(v, "")(h).ServeHTTP(res, bearerRequest("raw"))
	if res.Code != http.StatusForbidden {
		t.Fatalf("want 403, got %d", res.Code)
	}
	var env struct {
		Code string `json:"code"`
	}
	_ = json.Unmarshal(res.Body.Bytes(), &env)
	if env.Code != "AUTH_FORBIDDEN" {
		t.Fatalf("want AUTH_FORBIDDEN, got %s", env.Code)
	}
}

func TestRequireRoleAllowsEqualOrHigher(t *testing.T) {
	t.Parallel()
	v := stubVerifier{id: Identity{UserID: "u-9", Role: rolex.Admin}}
	h := RequireAuth(v, "")(RequireRole(rolex.Admin)(http.NotFoundHandler()))
	res := httptest.NewRecorder()
	h.ServeHTTP(res, bearerRequest("raw"))
	// NotFoundHandler's 404 proves the chain let the request through.
	if res.Code != http.StatusNotFound {
		t.Fatalf("want pass-through 404, got %d", res.Code)
	}
}
