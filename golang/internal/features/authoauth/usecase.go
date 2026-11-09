package authoauth

import (
	"context"
	"strings"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
	"github.com/<scope>/<app>/internal/shared/clockx"
	"github.com/<scope>/<app>/internal/shared/idgen"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// Config carries the social-login settings read from the <APP>_* env.
type Config struct {
	GoogleClientID     string
	GoogleEnabled      bool
	GitHubClientID     string
	GitHubClientSecret string
	GitHubCallbackURL  string
	GitHubEnabled      bool
	FrontendSuccess    string // <APP>_AUTH_FRONTEND_SUCCESS
	Offline            bool   // INV-T7
	AccessTTL          time.Duration
	RefreshTTL         time.Duration
}

// UseCase implements the google one-tap and github oauth flows on top of
// the session plumbing (same Result/session shape as the email login).
type UseCase struct {
	store        Store
	google       GoogleVerifier
	github       GitHubClient
	states       LinkState
	minter       TokenMinter
	clock        clockx.Clock
	accessTTL    time.Duration
	refreshTTL   time.Duration
	googleID     string
	githubID     string
	githubSecret string
	githubRedir  string
	frontend     string
	googleOn     bool
	githubOn     bool
	offline      bool
}

// New wires the use-case; nil clock falls back to the system clock.
func New(store Store, google GoogleVerifier, github GitHubClient, states LinkState,
	minter TokenMinter, clock clockx.Clock, c Config) *UseCase {
	if clock == nil {
		clock = clockx.System{}
	}
	return &UseCase{
		store: store, google: google, github: github, states: states,
		minter: minter, clock: clock, accessTTL: c.AccessTTL, refreshTTL: c.RefreshTTL,
		googleID: c.GoogleClientID, githubID: c.GitHubClientID,
		githubSecret: c.GitHubClientSecret, githubRedir: c.GitHubCallbackURL,
		frontend: c.FrontendSuccess, googleOn: c.GoogleEnabled, githubOn: c.GitHubEnabled,
		offline: c.Offline,
	}
}

// Google verifies the One-Tap credential and issues the session: google-id
// match, else email match (verify + link), else provision.
func (uc *UseCase) Google(ctx context.Context, credential, ip, ua string) (Result, error) {
	if uc.offline || !uc.googleOn || uc.googleID == "" {
		return Result{}, ErrGoogleNotConfigured
	}
	id, err := uc.google.Verify(ctx, credential, uc.googleID)
	if err != nil {
		return Result{}, ErrInvalidGoogleToken
	}
	user, err := uc.findOrLinkGoogle(ctx, id)
	if err != nil {
		return Result{}, err
	}
	return uc.startSession(ctx, user, ip, ua)
}

func (uc *UseCase) findOrLinkGoogle(ctx context.Context, id oauthx.GoogleIdentity) (User, error) {
	// Provider emails resolve against the stored lowercase convention (the
	// password flows normalize before every lookup/store).
	email := strings.ToLower(strings.TrimSpace(id.Email))
	user, found, err := uc.store.GetUserByGoogleID(ctx, id.Sub)
	if err != nil {
		return User{}, err
	}
	if found {
		return user, nil
	}
	user, found, err = uc.store.GetUserByEmail(ctx, email)
	if err != nil {
		return User{}, err
	}
	if found {
		if user.GoogleID != "" { // a different Google account owns this email
			return User{}, ErrGoogleEmailLinked
		}
		if err := uc.store.LinkGoogle(ctx, user.ID, id.Sub, id.Picture); err != nil {
			return User{}, err
		}
		user.GoogleID, user.IsVerified = id.Sub, true
		if user.Picture == "" {
			user.Picture = id.Picture
		}
		return user, nil
	}
	first, last := googleNames(id)
	fresh, err := newOAuthUser(email, first, last, id.Picture, id.Sub, "")
	if err != nil {
		return User{}, err
	}
	if err := uc.store.CreateUser(ctx, fresh); err != nil {
		return User{}, err
	}
	return fresh, nil
}

// GithubLoginURL returns the authorize redirect for the plain login flow
// (state passthrough); any refusal is a config problem.
func (uc *UseCase) GithubLoginURL(state string) (string, error) {
	if err := uc.githubGate(); err != nil {
		return "", err
	}
	return uc.github.Authorize(uc.githubID, uc.githubRedir, state), nil
}

// GithubConnectURL returns the authorize URL carrying state=link:<jwt> so
// the callback links instead of logging in (authenticated callers only).
func (uc *UseCase) GithubConnectURL(userID string) (string, error) {
	if err := uc.githubGate(); err != nil {
		return "", err
	}
	token, err := uc.states.MintLink(userID)
	if err != nil {
		return "", err
	}
	return uc.github.Authorize(uc.githubID, uc.githubRedir, LinkStatePrefix+token), nil
}

// GithubLogin processes the login callback: exchange code, resolve the
// real email, then github-id match (refresh identity) → email match (link)
// → provision.
func (uc *UseCase) GithubLogin(ctx context.Context, code, ip, ua string) (Result, error) {
	if err := uc.githubGate(); err != nil {
		return Result{}, err
	}
	gh, err := uc.github.Exchange(ctx, code, uc.githubID, uc.githubSecret, uc.githubRedir)
	if err != nil {
		return Result{}, err
	}
	user, err := uc.findOrLinkGithub(ctx, gh)
	if err != nil {
		return Result{}, err
	}
	return uc.startSession(ctx, user, ip, ua)
}

// GithubLink attaches the freshly exchanged github id to the user named by
// the link-state JWT; the link flow changes nothing else.
func (uc *UseCase) GithubLink(ctx context.Context, stateJWT, code string) error {
	if err := uc.githubGate(); err != nil {
		return err
	}
	userID, err := uc.states.ParseLink(stateJWT)
	if err != nil {
		return ErrInvalidLinkState
	}
	gh, err := uc.github.Exchange(ctx, code, uc.githubID, uc.githubSecret, uc.githubRedir)
	if err != nil {
		return err
	}
	if other, found, err := uc.store.GetUserByGithubID(ctx, gh.ID); err != nil {
		return err
	} else if found && other.ID != userID {
		return ErrGithubAlreadyLinked
	}
	target, found, err := uc.store.GetUserByID(ctx, userID)
	if err != nil {
		return err
	}
	if !found {
		return ErrUserNotFound
	}
	return uc.store.GithubOnly(ctx, target.ID, gh.ID)
}

// findOrLinkGithub mirrors reference: existing-by-github-id users get their
// identity refreshed (noreply email upgraded, avatar when empty, verified
// mark); email collisions are linked; strangers are provisioned.
func (uc *UseCase) findOrLinkGithub(ctx context.Context, gh oauthx.GitHubUser) (User, error) {
	// Provider emails resolve against the stored lowercase convention (the
	// password flows normalize before every lookup/store).
	ghEmail := strings.ToLower(strings.TrimSpace(gh.Email))
	user, found, err := uc.store.GetUserByGithubID(ctx, gh.ID)
	if err != nil {
		return User{}, err
	}
	if found {
		email := user.Email
		if isNoreply(user.Email) && ghEmail != "" && !isNoreply(ghEmail) {
			email = ghEmail
		}
		if err := uc.store.SaveGithubLogin(ctx, user.ID, email, gh.AvatarURL); err != nil {
			return User{}, err
		}
		user.Email, user.IsVerified = email, true
		if user.Picture == "" {
			user.Picture = gh.AvatarURL
		}
		return user, nil
	}
	user, found, err = uc.store.GetUserByEmail(ctx, ghEmail)
	if err != nil {
		return User{}, err
	}
	if found {
		if err := uc.store.LinkGithub(ctx, user.ID, gh.ID, gh.AvatarURL); err != nil {
			return User{}, err
		}
		user.GithubID, user.IsVerified = gh.ID, true
		if user.Picture == "" {
			user.Picture = gh.AvatarURL
		}
		return user, nil
	}
	first, last := githubNames(gh)
	fresh, err := newOAuthUser(ghEmail, first, last, gh.AvatarURL, "", gh.ID)
	if err != nil {
		return User{}, err
	}
	if err := uc.store.CreateUser(ctx, fresh); err != nil {
		return User{}, err
	}
	return fresh, nil
}

// githubGate is the one refusal rule of every github route.
func (uc *UseCase) githubGate() error {
	if uc.offline || !uc.githubOn || uc.githubID == "" || uc.githubSecret == "" {
		return ErrGithubNotConfigured
	}
	return nil
}

// startSession gates account state, stores the refresh session and mints
// the access token (jti == session id, exactly like the email login).
func (uc *UseCase) startSession(ctx context.Context, user User, ip, ua string) (Result, error) {
	switch {
	case user.IsBlocked:
		return Result{}, ErrAccountBlocked
	case !user.IsActive:
		return Result{}, ErrAccountInactive
	}
	refresh, err := idgen.NewToken()
	if err != nil {
		return Result{}, err
	}
	session := SessionRow{
		ID: idgen.UUIDGen{}.NewID(), UserID: user.ID, RefreshToken: refresh,
		IPAddress: ip, UserAgent: ua,
		ExpiresAt: uc.clock.Now().Add(uc.refreshTTL), IsActive: true,
	}
	if err := uc.store.CreateSession(ctx, session); err != nil {
		return Result{}, err
	}
	raw, exp, err := uc.minter.Mint(authx.Identity{
		UserID: user.ID, Email: user.Email, Role: rolex.Role(user.Role), JTI: session.ID,
	}, uc.accessTTL)
	if err != nil {
		return Result{}, err
	}
	return Result{AccessToken: raw, RefreshToken: refresh, ExpiresAt: exp}, nil
}

// newOAuthUser builds the provisioned social user: auto-verified, COMPANY
// user type (design default), and an unusable random password (the column
// is NOT NULL; the value is never handed to anyone).
func newOAuthUser(email, first, last, picture, googleID, githubID string) (User, error) {
	password, err := idgen.NewToken()
	if err != nil {
		return User{}, err
	}
	return User{
		ID: idgen.UUIDGen{}.NewID(), Email: email,
		FirstName: first, LastName: last, Password: password, Picture: picture,
		Role: "USER", UserType: "COMPANY", Language: "en",
		GoogleID: googleID, GithubID: githubID,
		IsActive: true, IsVerified: true,
	}, nil
}

// googleNames applies the reference defaults: given/family fall back to
// "User"/"Google" when the profile omits them.
func googleNames(id oauthx.GoogleIdentity) (first, last string) {
	first = strings.TrimSpace(id.GivenName)
	if first == "" {
		first = "User"
	}
	last = strings.TrimSpace(id.FamilyName)
	if last == "" {
		last = "Google"
	}
	return first, last
}

// githubNames splits the display name (login fallback) like reference.
func githubNames(gh oauthx.GitHubUser) (first, last string) {
	name := gh.Name
	if name == "" {
		name = gh.Login
	}
	parts := strings.SplitN(strings.TrimSpace(name), " ", 2)
	first = strings.TrimSpace(parts[0])
	if first == "" {
		first = "User"
	}
	last = "GitHub"
	if len(parts) > 1 {
		if v := strings.TrimSpace(parts[1]); v != "" {
			last = v
		}
	}
	return first, last
}

// isNoreply reports a GitHub noreply placeholder address.
func isNoreply(email string) bool {
	return strings.Contains(email, "noreply.github.com")
}

// frontendBase is the registered frontend success URL (trailing slash trimmed).
func (uc *UseCase) frontendBase() string {
	return strings.TrimRight(uc.frontend, "/")
}

// linkSuccessURL sends a completed link flow back to the profile page.
func (uc *UseCase) linkSuccessURL() string {
	return strings.TrimSuffix(uc.frontendBase(), authSuccessSuffix) + linkSuccessRedirect
}
