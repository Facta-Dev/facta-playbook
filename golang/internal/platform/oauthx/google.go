// Package oauthx is the OAuth infrastructure: Google One-Tap ID-token
// verification (RS256 against Google's published JWKS), the GitHub
// authorization-code client, and the github_link state-JWT codec. Offline
// mode (<APP>_OFFLINE, INV-T7) refuses before any socket is opened with a
// config-style error.
package oauthx

import (
	"context"
	"crypto/rsa"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/big"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// ErrOffline is the config-style refusal whenever a socket would be opened
// in offline mode (INV-T7).
var ErrOffline = errors.New("oauthx: offline mode forbids network calls")

// ErrInvalidIDToken marks a Google credential that failed verification.
var ErrInvalidIDToken = errors.New("oauthx: invalid google id token")

// googleJWKSURL is Google's public signing-key set.
const googleJWKSURL = "https://www.googleapis.com/oauth2/v3/certs"

// GoogleIdentity is the verified bearer of a One-Tap credential.
type GoogleIdentity struct {
	Sub        string
	Email      string
	GivenName  string
	FamilyName string
	Picture    string
}

// GoogleVerifier fetches (and caches honoring max-age) Google's JWKS and
// verifies RS256 ID tokens: kid → cached key, aud == client ID, iss ==
// accounts.google.com, expiry, and a verified email.
type GoogleVerifier struct {
	JWKSURL string       // overridable in tests; "" means Google's endpoint
	Offline bool         // INV-T7: refuse before any socket
	Client  *http.Client // nil => default 15s-timeout client

	mu         sync.Mutex
	keys       map[string]*rsa.PublicKey
	freshUntil time.Time
}

// NewGoogleVerifier builds the verifier; jwksURL "" selects Google's certs.
func NewGoogleVerifier(jwksURL string, offline bool) *GoogleVerifier {
	if jwksURL == "" {
		jwksURL = googleJWKSURL
	}
	return &GoogleVerifier{JWKSURL: jwksURL, Offline: offline}
}

// googleClaims is the subset of the Google ID token this slice consumes.
type googleClaims struct {
	Email         string `json:"email"`
	EmailVerified bool   `json:"email_verified"`
	GivenName     string `json:"given_name"`
	FamilyName    string `json:"family_name"`
	Picture       string `json:"picture"`
	jwt.RegisteredClaims
}

// Verify checks the RS256 signature (key from the max-age-cached JWKS),
// audience, issuer (both Google spellings), expiry, and requires a verified
// email.
func (v *GoogleVerifier) Verify(ctx context.Context, credential, clientID string) (GoogleIdentity, error) {
	if v.Offline {
		return GoogleIdentity{}, ErrOffline
	}
	var c googleClaims
	tok, err := jwt.ParseWithClaims(credential, &c, func(t *jwt.Token) (any, error) {
		kid, _ := t.Header["kid"].(string)
		return v.publicKey(ctx, kid)
	},
		jwt.WithValidMethods([]string{"RS256"}),
		jwt.WithAudience(clientID),
		jwt.WithExpirationRequired(),
		jwt.WithLeeway(time.Minute),
	)
	if err != nil || !tok.Valid {
		return GoogleIdentity{}, fmt.Errorf("%w: %v", ErrInvalidIDToken, err)
	}
	if c.Issuer != "accounts.google.com" && c.Issuer != "https://accounts.google.com" {
		return GoogleIdentity{}, fmt.Errorf("%w: unexpected issuer %q", ErrInvalidIDToken, c.Issuer)
	}
	if c.Subject == "" || c.Email == "" || !c.EmailVerified {
		return GoogleIdentity{}, fmt.Errorf("%w: missing verified email or subject", ErrInvalidIDToken)
	}
	return GoogleIdentity{
		Sub: c.Subject, Email: c.Email,
		GivenName: c.GivenName, FamilyName: c.FamilyName, Picture: c.Picture,
	}, nil
}

// publicKey resolves the token's kid from the cache, refreshing the JWKS
// when stale or unknown (Google rotates keys; an unknown kid forces one
// bypass of the cache before giving up).
func (v *GoogleVerifier) publicKey(ctx context.Context, kid string) (*rsa.PublicKey, error) {
	v.mu.Lock()
	defer v.mu.Unlock()
	if time.Now().After(v.freshUntil) {
		if err := v.refreshLocked(ctx); err != nil && len(v.keys) == 0 {
			return nil, err // first fetch must succeed; a stale set may still verify
		}
	}
	if key, ok := v.keys[kid]; ok {
		return key, nil
	}
	if err := v.refreshLocked(ctx); err != nil {
		return nil, err
	}
	if key, ok := v.keys[kid]; ok {
		return key, nil
	}
	return nil, fmt.Errorf("%w: unknown signing kid", ErrInvalidIDToken)
}

// refreshLocked refetches the JWKS and honors Cache-Control max-age.
func (v *GoogleVerifier) refreshLocked(ctx context.Context) error {
	client := v.Client
	if client == nil {
		client = &http.Client{Timeout: 15 * time.Second}
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, v.JWKSURL, nil)
	if err != nil {
		return err
	}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("oauthx: google jwks fetch: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("oauthx: google jwks fetch: status %d", resp.StatusCode)
	}
	body, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return fmt.Errorf("oauthx: google jwks read: %w", err)
	}
	var set struct {
		Keys []jwksKey `json:"keys"`
	}
	if err := json.Unmarshal(body, &set); err != nil {
		return fmt.Errorf("oauthx: google jwks parse: %w", err)
	}
	keys := make(map[string]*rsa.PublicKey, len(set.Keys))
	for _, k := range set.Keys {
		pub, err := k.rsaKey()
		if err != nil || pub == nil {
			continue
		}
		keys[k.Kid] = pub
	}
	if len(keys) == 0 {
		return errors.New("oauthx: google jwks has no rsa keys")
	}
	v.keys, v.freshUntil = keys, time.Now().Add(maxAge(resp.Header.Get("Cache-Control")))
	return nil
}

// jwksKey is one RSA entry of the published key set.
type jwksKey struct {
	Kid string `json:"kid"`
	Kty string `json:"kty"`
	N   string `json:"n"`
	E   string `json:"e"`
}

func (k jwksKey) rsaKey() (*rsa.PublicKey, error) {
	if k.Kty != "RSA" || k.N == "" || k.E == "" {
		return nil, nil
	}
	n, err := base64.RawURLEncoding.DecodeString(k.N)
	if err != nil {
		return nil, err
	}
	e, err := base64.RawURLEncoding.DecodeString(k.E)
	if err != nil {
		return nil, err
	}
	exp := new(big.Int).SetBytes(e)
	if exp.Int64() <= 0 || exp.BitLen() > 31 {
		return nil, fmt.Errorf("oauthx: bad rsa exponent for kid %q", k.Kid)
	}
	return &rsa.PublicKey{N: new(big.Int).SetBytes(n), E: int(exp.Int64())}, nil
}

// maxAge parses "max-age=N" out of Cache-Control; a missing or malformed
// value falls back to 5 minutes.
func maxAge(cacheControl string) time.Duration {
	const fallback = 5 * time.Minute
	for _, dir := range strings.Split(cacheControl, ",") {
		dir = strings.TrimSpace(dir)
		if value, ok := strings.CutPrefix(dir, "max-age="); ok {
			secs, err := strconv.Atoi(value)
			if err != nil || secs < 0 {
				return fallback
			}
			return time.Duration(secs) * time.Second
		}
	}
	return fallback
}
