// Package authx is the authentication kernel: identity in context, the
// HS256 JWT verifier/minter, password hashing (bcrypt over a peppered
// HMAC pre-hash), and the RequireAuth/RequireRole middlewares.
package authx

import (
	"context"
	"fmt"

	"github.com/golang-jwt/jwt/v5"

	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// Identity is the authenticated actor, extracted from verified claims.
type Identity struct {
	UserID string
	Email  string
	Role   rolex.Role
	JTI    string
}

type ctxKey struct{}

// WithIdentity stores the identity on the context.
func WithIdentity(ctx context.Context, id Identity) context.Context {
	return context.WithValue(ctx, ctxKey{}, id)
}

// IdentityFrom reads the identity set by the auth middleware.
func IdentityFrom(ctx context.Context) (Identity, bool) {
	id, ok := ctx.Value(ctxKey{}).(Identity)
	return id, ok
}

// Verifier parses and validates a raw token (impl: HS256Verifier).
type Verifier interface {
	Parse(raw string) (Identity, error)
}

type HS256Verifier struct {
	key    []byte
	issuer string
}

// NewHS256Verifier builds the verifier; issuer tags claims for hygiene.
func NewHS256Verifier(secret, issuer string) *HS256Verifier {
	return &HS256Verifier{key: []byte(secret), issuer: issuer}
}

type claims struct {
	Email string `json:"email"`
	Role  string `json:"role"`
	jwt.RegisteredClaims
}

// Parse verifies signature, issuer, and expiry, and rebuilds the Identity.
func (v *HS256Verifier) Parse(raw string) (Identity, error) {
	var c claims
	token, err := jwt.ParseWithClaims(raw, &c, func(t *jwt.Token) (any, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method %v", t.Header["alg"])
		}
		return v.key, nil
	}, jwt.WithValidMethods([]string{"HS256"}), jwt.WithIssuer(v.issuer))
	if err != nil || !token.Valid {
		return Identity{}, apperr.E(apperr.CodeUnauthorized, apperr.AuthInvalidToken, "invalid or expired token")
	}
	return Identity{
		UserID: c.Subject,
		Email:  c.Email,
		Role:   rolex.Role(c.Role),
		JTI:    c.ID,
	}, nil
}
