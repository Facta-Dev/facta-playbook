package oauthx

import (
	"crypto/rand"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"errors"
	"math/big"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// googleKey is the fixture RSA key; the JWKS server publishes it.
var googleKey, keyErr = rsa.GenerateKey(rand.Reader, 2048)

// testJWKS writes a one-key key set for kid.
func testJWKS(t *testing.T, w http.ResponseWriter, kid string) {
	t.Helper()
	if keyErr != nil {
		t.Fatalf("fixture key: %v", keyErr)
	}
	pub := &googleKey.PublicKey
	w.Header().Set("Cache-Control", "public, max-age=60")
	_ = json.NewEncoder(w).Encode(map[string]any{
		"keys": []map[string]string{{
			"kty": "RSA", "kid": kid,
			"n": base64.RawURLEncoding.EncodeToString(pub.N.Bytes()),
			"e": base64.RawURLEncoding.EncodeToString(big.NewInt(int64(pub.E)).Bytes()),
		}},
	})
}

// mintGoogle signs claims with the fixture key under kid; alg "HS256" mints
// a wrong-algorithm token (the verifier must refuse by method, not signature).
func mintGoogle(t *testing.T, claims jwt.MapClaims, kid, alg string) string {
	t.Helper()
	if keyErr != nil {
		t.Fatalf("fixture key: %v", keyErr)
	}
	tok := jwt.NewWithClaims(jwt.SigningMethodRS256, claims)
	tok.Header["kid"] = kid
	raw, err := tok.SignedString(googleKey)
	if alg == "HS256" {
		hs := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
		hs.Header["kid"] = kid
		raw, err = hs.SignedString([]byte("not-the-secret"))
	}
	if err != nil {
		t.Fatalf("mint: %v", err)
	}
	return raw
}

func googleTestClaims() jwt.MapClaims {
	return jwt.MapClaims{
		"iss": "https://accounts.google.com", "aud": "g-client", "sub": "123",
		"email": "a@ex.com", "email_verified": true,
		"exp": time.Now().Add(time.Hour).Unix(), "iat": time.Now().Unix(),
		"given_name": "Ada", "family_name": "Lovelace", "picture": "pic",
	}
}

func TestGoogleVerify(t *testing.T) {
	t.Parallel()
	fetches := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		fetches++
		testJWKS(t, w, "k1")
	}))
	defer srv.Close()

	v := NewGoogleVerifier(srv.URL, false)
	ctx := t.Context()

	id, err := v.Verify(ctx, mintGoogle(t, googleTestClaims(), "k1", ""), "g-client")
	if err != nil {
		t.Fatalf("verify: %v", err)
	}
	if id.Sub != "123" || id.Email != "a@ex.com" || id.GivenName != "Ada" || id.Picture != "pic" {
		t.Fatalf("bad identity: %+v", id)
	}
	if fetches != 1 {
		t.Fatalf("cache ignored: %d fetches", fetches)
	}

	// A cached still-fresh set must not refetch, but an unknown kid forces
	// exactly one bypass.
	if _, err := v.Verify(ctx, mintGoogle(t, googleTestClaims(), "k-other", ""), "g-client"); !errors.Is(err, ErrInvalidIDToken) {
		t.Fatalf("unknown kid: %v", err)
	}
	if fetches != 2 {
		t.Fatalf("kid bypass fetch missing: %d", fetches)
	}

	// Refusals: wrong aud, wrong iss, unverified email, HS256 alg.
	refused := googleTestClaims()
	refused["aud"] = "someone-else"
	if _, err := v.Verify(ctx, mintGoogle(t, refused, "k1", ""), "g-client"); !errors.Is(err, ErrInvalidIDToken) {
		t.Fatalf("aud check: %v", err)
	}
	refused = googleTestClaims()
	refused["iss"] = "https://evil.example"
	if _, err := v.Verify(ctx, mintGoogle(t, refused, "k1", ""), "g-client"); !errors.Is(err, ErrInvalidIDToken) {
		t.Fatalf("iss check: %v", err)
	}
	refused = googleTestClaims()
	refused["email_verified"] = false
	if _, err := v.Verify(ctx, mintGoogle(t, refused, "k1", ""), "g-client"); !errors.Is(err, ErrInvalidIDToken) {
		t.Fatalf("email_verified check: %v", err)
	}
	if _, err := v.Verify(ctx, mintGoogle(t, googleTestClaims(), "k1", "HS256"), "g-client"); !errors.Is(err, ErrInvalidIDToken) {
		t.Fatalf("alg check: %v", err)
	}

	// Offline refuses before any socket.
	offline := NewGoogleVerifier(srv.URL, true)
	if _, err := offline.Verify(ctx, mintGoogle(t, googleTestClaims(), "k1", ""), "g-client"); !errors.Is(err, ErrOffline) {
		t.Fatalf("offline refusal: %v", err)
	}
}

func githubServer(t *testing.T, profile map[string]any, emails any) *httptest.Server {
	t.Helper()
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/token":
			_ = r.ParseForm()
			if r.Form.Get("client_id") != "cid" || r.Form.Get("code") != "the-code" ||
				r.Form.Get("redirect_uri") != "http://cb" {
				t.Errorf("token form wrong: %v", r.Form)
				w.WriteHeader(http.StatusBadRequest)
				return
			}
			_ = json.NewEncoder(w).Encode(map[string]string{"access_token": "gh-t"})
		case "/user":
			if r.Header.Get("Authorization") != "Bearer gh-t" {
				t.Errorf("missing bearer")
				w.WriteHeader(http.StatusUnauthorized)
				return
			}
			_ = json.NewEncoder(w).Encode(profile)
		case "/emails":
			if emails == nil {
				w.WriteHeader(http.StatusNotFound)
				return
			}
			_ = json.NewEncoder(w).Encode(emails)
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	return srv
}

func githubClient(srv *httptest.Server) *GitHubClient {
	c := NewGitHubClient(false)
	c.TokenURL, c.UserURL, c.EmailsURL = srv.URL+"/token", srv.URL+"/user", srv.URL+"/emails"
	return c
}

func TestGithubExchange(t *testing.T) {
	t.Parallel()
	ctx := t.Context()

	// noreply profile + real primary email → the real one wins.
	srv := githubServer(t, map[string]any{
		"id": 1001, "login": "ada", "name": "Ada Lovelace",
		"email": "1001@users.noreply.github.com", "avatar_url": "av",
	}, []ghEmail{
		{Email: "1001@users.noreply.github.com", Verified: true},
		{Email: "ada@ex.com", Primary: true, Verified: true},
	})
	defer srv.Close()
	u, err := githubClient(srv).Exchange(ctx, "the-code", "cid", "sec", "http://cb")
	if err != nil {
		t.Fatalf("exchange: %v", err)
	}
	if u.ID != "1001" || u.Name != "Ada Lovelace" || u.Email != "ada@ex.com" || u.AvatarURL != "av" {
		t.Fatalf("bad user: %+v", u)
	}

	// A real profile email is kept even without /user/emails help.
	srv2 := githubServer(t, map[string]any{
		"id": 1002, "login": "bob", "email": "real@ex.com",
	}, []ghEmail{})
	defer srv2.Close()
	u, err = githubClient(srv2).Exchange(ctx, "the-code", "cid", "sec", "http://cb")
	if err != nil {
		t.Fatalf("exchange 2: %v", err)
	}
	if u.Email != "real@ex.com" {
		t.Fatalf("profile email not kept: %+v", u)
	}

	// No emails access at all → the id-based noreply fallback.
	srv3 := githubServer(t, map[string]any{"id": 1003, "login": "carol", "email": nil}, nil)
	defer srv3.Close()
	u, err = githubClient(srv3).Exchange(ctx, "the-code", "cid", "sec", "http://cb")
	if err != nil {
		t.Fatalf("exchange 3: %v", err)
	}
	if u.Email != "1003@users.noreply.github.com" {
		t.Fatalf("fallback email wrong: %+v", u)
	}

	// Token exchange failure surfaces GitHub's own error words.
	srv4 := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]string{
			"error": "bad_verification_code", "error_description": "unknown code",
		})
	}))
	defer srv4.Close()
	c := githubClient(srv4)
	if _, err := c.Exchange(ctx, "the-code", "cid", "sec", "http://cb"); err == nil ||
		!strings.Contains(err.Error(), "bad_verification_code") {
		t.Fatalf("exchange failure: %v", err)
	}

	// Offline refuses before any socket.
	offline := NewGitHubClient(true)
	if _, err := offline.Exchange(ctx, "the-code", "cid", "sec", "http://cb"); !errors.Is(err, ErrOffline) {
		t.Fatalf("offline refusal: %v", err)
	}
}

func TestLinkToken(t *testing.T) {
	t.Parallel()
	l := NewLinkToken("0123456789abcdef0123456789abcdef")
	state, err := l.MintLink("u-1")
	if err != nil {
		t.Fatalf("mint: %v", err)
	}
	if sub, err := l.ParseLink(state); err != nil || sub != "u-1" {
		t.Fatalf("roundtrip: %q %v", sub, err)
	}

	// A same-secret token without the purpose claim must not pass as state.
	forged := jwt.NewWithClaims(jwt.SigningMethodHS256, jwt.MapClaims{
		"sub": "u-1", "exp": time.Now().Add(time.Hour).Unix(),
	})
	forgedRaw, err := forged.SignedString(l.key)
	if err != nil {
		t.Fatalf("forge: %v", err)
	}
	if _, err := l.ParseLink(forgedRaw); !errors.Is(err, ErrInvalidLinkState) {
		t.Fatalf("purpose check: %v", err)
	}

	// Expired and wrong-algorithm states are refused, not honored.
	expired := &LinkToken{key: []byte("0123456789abcdef0123456789abcdef"), ttl: -time.Minute}
	state, err = expired.MintLink("u-1")
	if err != nil {
		t.Fatalf("mint expired: %v", err)
	}
	if _, err := l.ParseLink(state); !errors.Is(err, ErrInvalidLinkState) {
		t.Fatalf("expiry check: %v", err)
	}
	if _, err := l.ParseLink("garbage"); !errors.Is(err, ErrInvalidLinkState) {
		t.Fatalf("garbage check: %v", err)
	}
}
