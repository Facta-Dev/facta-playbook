package authx

import (
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// Minter issues access tokens (impl: HS256Minter).
type Minter interface {
	Mint(id Identity, ttl time.Duration) (token string, exp time.Time, err error)
}

// HS256Minter signs HS256 JWTs with the shared secret; issuer tags tokens
// with "<app>-api" (rename-substituted).
type HS256Minter struct {
	key    []byte
	issuer string
}

// NewHS256Minter builds the minter.
func NewHS256Minter(secret, issuer string) *HS256Minter {
	return &HS256Minter{key: []byte(secret), issuer: issuer}
}

// Mint signs {sub, email, role, jti} plus std exp/iat/iss; aud stays unset
// (single-audience API; verifier accepts any aud).
func (m *HS256Minter) Mint(id Identity, ttl time.Duration) (string, time.Time, error) {
	now := time.Now()
	exp := now.Add(ttl)
	claims := jwt.MapClaims{
		"sub":   id.UserID,
		"email": id.Email,
		"role":  string(id.Role),
		"jti":   id.JTI,
		"iat":   now.Unix(),
		"exp":   exp.Unix(),
		"iss":   m.issuer,
	}
	token, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString(m.key)
	if err != nil {
		return "", time.Time{}, err
	}
	return token, exp, nil
}
