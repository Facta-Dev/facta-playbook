// linktoken.go: the ?state= JWT of the GitHub account-linking flow
// ({sub, purpose:github_link}, HS256, shared API secret). minted by the
// connect-url endpoint, verified back on the callback; the purpose claim
// keeps access tokens from double-clicking as link states.
package oauthx

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// linkStateTTL matches reference LINK_STATE_EXP_SEC (600s).
const linkStateTTL = 10 * time.Minute

// githubLinkPurpose is the purpose claim guarding the flow.
const githubLinkPurpose = "github_link"

// ErrInvalidLinkState marks a state JWT that failed verification (wrong
// signature, expired, missing/wrong purpose).
var ErrInvalidLinkState = errors.New("oauthx: invalid link state")

// LinkToken mints and parses the link-state JWT with the shared secret.
type LinkToken struct {
	key []byte
	ttl time.Duration
}

// NewLinkToken builds the codec over the API's HS256 secret.
func NewLinkToken(secret string) *LinkToken {
	return &LinkToken{key: []byte(secret), ttl: linkStateTTL}
}

type linkClaims struct {
	Purpose string `json:"purpose"`
	jwt.RegisteredClaims
}

// MintLink signs {sub, purpose, exp}.
func (l *LinkToken) MintLink(userID string) (string, error) {
	claims := linkClaims{
		Purpose: githubLinkPurpose,
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userID,
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(l.ttl)),
		},
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString(l.key)
}

// ParseLink returns the subject of a valid link-state token; anything else
// (wrong algorithm, expired, wrong/missing purpose, empty subject) is a
// refusal.
func (l *LinkToken) ParseLink(state string) (string, error) {
	var c linkClaims
	tok, err := jwt.ParseWithClaims(state, &c, func(*jwt.Token) (any, error) { return l.key, nil },
		jwt.WithValidMethods([]string{"HS256"}),
		jwt.WithExpirationRequired(),
	)
	if err != nil || !tok.Valid {
		return "", ErrInvalidLinkState
	}
	if c.Purpose != githubLinkPurpose || c.Subject == "" {
		return "", ErrInvalidLinkState
	}
	return c.Subject, nil
}
