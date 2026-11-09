// Package idgen generates identifiers: UUIDs for rows and opaque,
// unguessable tokens for session refresh.
package idgen

import (
	"crypto/rand"
	"encoding/base64"
	"math/big"

	"github.com/google/uuid"
)

var (
	digits   = []byte("0123456789")
	digitMax = big.NewInt(int64(len(digits)))
)

type Generator interface{ NewID() string }

type UUIDGen struct{}

func (UUIDGen) NewID() string { return uuid.NewString() }

// NewToken returns a 64-char URL-safe opaque token (48 random bytes),
// suitable for session refresh tokens.
func NewToken() (string, error) {
	b := make([]byte, 48)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return base64.RawURLEncoding.EncodeToString(b), nil
}

// NewCode returns a n-digit numeric code as a string (crypto/rand backed).
func NewCode(n int) (string, error) {
	b := make([]byte, n)
	for i := range b {
		v, err := rand.Int(rand.Reader, digitMax)
		if err != nil {
			return "", err
		}
		b[i] = digits[v.Int64()]
	}
	return string(b), nil
}
