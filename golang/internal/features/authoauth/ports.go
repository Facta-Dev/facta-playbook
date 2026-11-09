package authoauth

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
)

// GoogleVerifier verifies One-Tap credentials (impl: oauthx, RS256 over the
// cached Google JWKS).
type GoogleVerifier interface {
	Verify(ctx context.Context, credential, clientID string) (oauthx.GoogleIdentity, error)
}

// GitHubClient builds the authorize URL and exchanges codes (impl: oauthx).
type GitHubClient interface {
	Authorize(clientID, redirectURI, state string) string
	Exchange(ctx context.Context, code, clientID, clientSecret, redirectURI string) (oauthx.GitHubUser, error)
}

// LinkState mints/parses the ?state= link JWT ({sub, purpose:github_link};
// impl: oauthx.LinkToken over the shared HS256 secret).
type LinkState interface {
	MintLink(userID string) (string, error)
	ParseLink(state string) (string, error)
}

// TokenMinter issues access JWTs (platform/authx satisfies it).
type TokenMinter interface {
	Mint(id authx.Identity, ttl time.Duration) (token string, exp time.Time, err error)
}

// Store persists users + sessions for the social flows (impl: postgres.go).
type Store interface {
	GetUserByGoogleID(ctx context.Context, googleID string) (User, bool, error)
	GetUserByGithubID(ctx context.Context, githubID string) (User, bool, error)
	GetUserByEmail(ctx context.Context, email string) (User, bool, error)
	GetUserByID(ctx context.Context, id string) (User, bool, error)
	// CreateUser provisions the social user (auto-verified, unusable password).
	CreateUser(ctx context.Context, u User) error
	// LinkGoogle/LinkGithub attach the provider id to an email-matched user
	// (verifying it and filling the picture when empty, SQL-side).
	LinkGoogle(ctx context.Context, userID, providerID, picture string) error
	LinkGithub(ctx context.Context, userID, providerID, picture string) error
	// GithubOnly is the link-flow write: attach the github id, nothing else.
	GithubOnly(ctx context.Context, userID, githubID string) error
	// SaveGithubLogin applies the login-flow identity refresh (email upgrade
	// decided in Go; picture-if-empty and verified mark SQL-side).
	SaveGithubLogin(ctx context.Context, userID, email, picture string) error
	CreateSession(ctx context.Context, s SessionRow) error
}
