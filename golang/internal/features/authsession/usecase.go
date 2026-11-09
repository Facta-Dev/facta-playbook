package authsession

import (
	"context"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/clockx"
	"github.com/<scope>/<app>/internal/shared/idgen"
	"github.com/<scope>/<app>/internal/shared/rolex"
)

// UseCase implements login / refresh / logout / me.
type UseCase struct {
	store         Store
	minter        TokenMinter
	clock         clockx.Clock
	pepper        string
	accessTTL     time.Duration
	refreshTTL    time.Duration
	maxLoginRetry int
}

// New wires the use-case; nil clock => system clock.
func New(store Store, minter TokenMinter, clock clockx.Clock, pepper string,
	accessTTL, refreshTTL time.Duration, maxLoginRetry int) *UseCase {
	if clock == nil {
		clock = clockx.System{}
	}
	return &UseCase{store: store, minter: minter, clock: clock, pepper: pepper,
		accessTTL: accessTTL, refreshTTL: refreshTTL, maxLoginRetry: maxLoginRetry}
}

// LoginInput carries request + request metadata for the session row.
type LoginInput struct {
	Email, Password, IPAddress, UserAgent string
}

// Result is what the HTTP adapter turns into api.TokenPair.
type Result struct {
	AccessToken  string
	RefreshToken string
	ExpiresAt    time.Time
}

// Login counts failures with the retry lock, and — only after the password
// verifies (reference parity: the state gate codes are unreachable with a
// wrong password, so a probe cannot reveal an account's existence or state;
// specs/features/authsession.feature promises exactly this) — the account
// state gates, then issues the token pair. A wrong password on an account
// provisioned through Google/GitHub guides toward that provider (reference
// SignedUpWithSocial parity) and still counts against the lock.
func (uc *UseCase) Login(ctx context.Context, in LoginInput) (Result, error) {
	user, found, err := uc.store.GetUserByEmail(ctx, in.Email)
	if err != nil {
		return Result{}, err
	}
	if !found {
		return Result{}, ErrInvalidCredentials
	}
	if authx.CheckPassword(user.PasswordHash, in.Password, uc.pepper) != nil {
		blocked, err := uc.store.AfterFailedLogin(ctx, user.ID, uc.maxLoginRetry)
		if err != nil {
			return Result{}, err
		}
		if blocked {
			return Result{}, ErrAccountBlocked
		}
		if user.GoogleLinked || user.GithubLinked {
			return Result{}, ErrSignedUpWithSocial.WithDetail(socialGuidance(user.GoogleLinked, user.GithubLinked))
		}
		return Result{}, ErrInvalidCredentials
	}
	switch {
	case user.IsBlocked:
		return Result{}, ErrAccountBlocked
	case !user.IsActive:
		return Result{}, ErrAccountInactive
	case !user.IsVerified:
		return Result{}, ErrEmailNotVerified
	}
	if err := uc.store.AfterSuccessfulLogin(ctx, user.ID); err != nil {
		return Result{}, err
	}
	session, err := uc.newSession(in.IPAddress, in.UserAgent, user.ID)
	if err != nil {
		return Result{}, err
	}
	if err := uc.store.CreateSession(ctx, session); err != nil {
		return Result{}, err
	}
	return uc.mintFor(user, session.ID, session.RefreshToken)
}

// Refresh re-checks the account state, then rotates the session row in
// place: the row keeps its id and expires_at (fixed absolute lifetime — a
// session never lives past the refresh TTL set at login, reference
// update_tokens parity), and the compare-and-swap on the old token rejects a
// replayed token (0 affected rows). mintFor keeps issuing jti == session id,
// so the identity survives rotation unchanged.
func (uc *UseCase) Refresh(ctx context.Context, token, ip, ua string) (Result, error) {
	su, found, err := uc.store.ActiveSessionByToken(ctx, token)
	if err != nil {
		return Result{}, err
	}
	if !found {
		return Result{}, ErrInvalidRefresh
	}
	switch {
	case su.IsBlocked:
		_, _ = uc.store.DeactivateSession(ctx, su.ID)
		return Result{}, ErrAccountBlocked
	case !su.UserActive:
		_, _ = uc.store.DeactivateSession(ctx, su.ID)
		return Result{}, ErrAccountInactive
	case !su.IsVerified:
		_, _ = uc.store.DeactivateSession(ctx, su.ID)
		return Result{}, ErrEmailNotVerified
	}
	newToken, err := idgen.NewToken()
	if err != nil {
		return Result{}, err
	}
	rotated := SessionRow{
		ID:           su.ID,
		UserID:       su.UserID,
		RefreshToken: newToken,
		ExpiresAt:    su.ExpiresAt,
		IPAddress:    ip,
		UserAgent:    ua,
		IsActive:     true,
	}
	claimed, err := uc.store.RotateSession(ctx, su.ID, su.RefreshToken, rotated)
	if err != nil {
		return Result{}, err
	}
	if !claimed {
		return Result{}, ErrInvalidRefresh
	}
	return uc.mintFor(UserRow{
		ID: su.UserID, Email: su.Email, Role: su.Role,
		IsActive: su.UserActive, IsVerified: su.IsVerified,
	}, su.ID, newToken)
}

// Logout revokes the given refresh token for this user (idempotent).
func (uc *UseCase) Logout(ctx context.Context, userID, refreshToken string) error {
	return uc.store.DeactivateByTokenForUser(ctx, userID, refreshToken)
}

// Me returns the profile of the authenticated user.
func (uc *UseCase) Me(ctx context.Context, userID string) (UserRow, error) {
	user, ok, err := uc.store.GetUserByID(ctx, userID)
	if err != nil {
		return UserRow{}, err
	}
	if !ok {
		return UserRow{}, ErrUserNotFound
	}
	return user, nil
}

// UpdateMeInput carries the optional profile fields; nil = leave unchanged,
// ” sets the empty string (never NULL). Mirrors the reference
// UpdateMeRequest with the geo fields (country/city) dropped — the template
// has no countries/cities tables.
type UpdateMeInput struct {
	FirstName *string
	LastName  *string
	Phone     *string
	Picture   *string
	Language  *string
	Address   *string
	Postal    *string
}

// UpdateMe applies the non-nil fields and re-reads the profile (reference
// update-then-GetMe parity). Unknown id → ErrUserNotFound.
func (uc *UseCase) UpdateMe(ctx context.Context, userID string, input UpdateMeInput) (UserRow, error) {
	user, ok, err := uc.store.UpdateMe(ctx, userID, input)
	if err != nil {
		return UserRow{}, err
	}
	if !ok {
		return UserRow{}, ErrUserNotFound
	}
	return user, nil
}

// Sessions lists the user's sessions, newest activity first (active-only by
// default, reference /user/auth-session/all parity).
func (uc *UseCase) Sessions(ctx context.Context, userID string, activeOnly bool) ([]SessionRow, error) {
	return uc.store.ListSessionsByUser(ctx, userID, activeOnly)
}

// CurrentSession returns the user's most recent active session; the
// reference API's session_id hint is unused there too (reader_sqla ignores
// it), so it is not carried.
func (uc *UseCase) CurrentSession(ctx context.Context, userID string) (SessionRow, bool, error) {
	return uc.store.CurrentSession(ctx, userID)
}

// RevokeCurrent claims one active session of the user (CAS on is_active):
// false when sessionID is not among the user's active sessions.
func (uc *UseCase) RevokeCurrent(ctx context.Context, userID, sessionID string) (bool, error) {
	if sessionID == "" {
		return false, nil
	}
	return uc.store.RevokeSessionForUser(ctx, userID, sessionID)
}

// RevokeAll deactivates every active session of the user and returns how
// many were claimed.
func (uc *UseCase) RevokeAll(ctx context.Context, userID string) (int, error) {
	return uc.store.RevokeAllForUser(ctx, userID)
}

const maxAnomalyWindow = 30 * 24 * time.Hour

// Anomalies is the admin session report: expired-but-active rows first,
// then users whose active sessions within the window span more distinct
// IPs than the SQL threshold (reference reader aggregate parity).
func (uc *UseCase) Anomalies(ctx context.Context, window time.Duration) ([]Anomaly, error) {
	if window <= 0 || window > maxAnomalyWindow {
		return nil, apperr.E(apperr.CodeUnprocessable, apperr.Validation, "window out of range")
	}
	expired, err := uc.store.CountExpiredActive(ctx)
	if err != nil {
		return nil, err
	}
	var out []Anomaly
	if expired > 0 {
		out = append(out, Anomaly{Type: "expired_but_active", Count: expired})
	}
	rows, err := uc.store.MultiIPSessions(ctx, uc.clock.Now().Add(-window))
	if err != nil {
		return nil, err
	}
	if len(rows) > 0 {
		ids := make([]string, 0, len(rows))
		for _, r := range rows {
			ids = append(ids, r.UserID)
		}
		out = append(out, Anomaly{
			Type: "multi_ip_sessions", Count: len(rows),
			Details: map[string]any{"user_ids": ids},
		})
	}
	return out, nil
}

func (uc *UseCase) newSession(ip, ua, userID string) (SessionRow, error) {
	token, err := idgen.NewToken()
	if err != nil {
		return SessionRow{}, err
	}
	return SessionRow{
		ID:           idgen.UUIDGen{}.NewID(),
		UserID:       userID,
		RefreshToken: token,
		ExpiresAt:    uc.clock.Now().Add(uc.refreshTTL),
		IPAddress:    ip,
		UserAgent:    ua,
		IsActive:     true,
	}, nil
}

// socialGuidance renders the wrong-password guidance for an account whose
// identity came from Google/GitHub (reference message parity).
func socialGuidance(google, github bool) string {
	switch {
	case google && github:
		return "This account was created with Google and GitHub. Use one of those to sign in, or reset your password."
	case google:
		return "This account was created with Google. Use Google to sign in, or reset your password to set a password."
	default:
		return "This account was created with GitHub. Use GitHub to sign in, or reset your password to set a password."
	}
}

// mintFor issues the access JWT with jti == session ID; change-password's
// "keep the current session" matches on exactly this.
func (uc *UseCase) mintFor(user UserRow, sessionID, refreshToken string) (Result, error) {
	id := authx.Identity{UserID: user.ID, Email: user.Email, Role: rolex.Role(user.Role), JTI: sessionID}
	raw, exp, err := uc.minter.Mint(id, uc.accessTTL)
	if err != nil {
		return Result{}, err
	}
	return Result{AccessToken: raw, RefreshToken: refreshToken, ExpiresAt: exp}, nil
}
