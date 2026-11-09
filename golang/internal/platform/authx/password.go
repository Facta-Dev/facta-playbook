package authx

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"

	"golang.org/x/crypto/bcrypt"
)

// bcryptCost mirrors ctx0 parity (gensalt 10).
const bcryptCost = 10

// preHash conditions the password before bcrypt. With a pepper: HMAC-SHA256
// keyed by the pepper, base64-encoded — this defeats bcrypt's 72-byte password
// limit and any NUL truncation, and keeps the pepper secret even from a
// stolen users table. Without a pepper the raw password feeds bcrypt directly
// (documented ceiling: bcrypt silently truncates past 72 bytes).
func preHash(password, pepper string) []byte {
	if pepper == "" {
		return []byte(password)
	}
	mac := hmac.New(sha256.New, []byte(pepper))
	mac.Write([]byte(password))
	return []byte(base64.StdEncoding.EncodeToString(mac.Sum(nil)))
}

// HashPassword builds the stored bcrypt hash (cost 10) for a password.
func HashPassword(password, pepper string) (string, error) {
	b, err := bcrypt.GenerateFromPassword(preHash(password, pepper), bcryptCost)
	return string(b), err
}

// CheckPassword reports whether the password matches the stored hash; a
// mismatch (or malformed hash) returns an error for the use-case to map.
func CheckPassword(hash, password, pepper string) error {
	return bcrypt.CompareHashAndPassword([]byte(hash), preHash(password, pepper))
}
