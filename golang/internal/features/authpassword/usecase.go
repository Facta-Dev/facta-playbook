package authpassword

import (
	"context"
	"crypto/subtle"
	"strings"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/email"
	"github.com/<scope>/<app>/internal/shared/clockx"
	"github.com/<scope>/<app>/internal/shared/idgen"
)

// UseCase implements forgot / validate / reset / change.
type UseCase struct {
	store            Store
	emails           EmailEnqueuer
	clock            clockx.Clock
	pepper           string
	resetT           time.Duration // code lifetime (30m)
	window           time.Duration // rate-limit window (5m)
	maxResets        int           // reset codes allowed per window
	logOutAllOnReset bool
}

// New wires the use-case; nil clock falls back to the system clock.
func New(store Store, emails EmailEnqueuer, clock clockx.Clock, pepper string,
	resetT, window time.Duration, maxResets int, logOutAllOnReset bool) *UseCase {
	if clock == nil {
		clock = clockx.System{}
	}
	return &UseCase{store: store, emails: emails, clock: clock, pepper: pepper,
		resetT: resetT, window: window, maxResets: maxResets,
		logOutAllOnReset: logOutAllOnReset}
}

// Forgot rate-limits, then queues a fresh reset code (no existence leak).
func (uc *UseCase) Forgot(ctx context.Context, email string) error {
	email = strings.ToLower(strings.TrimSpace(email))
	now := uc.clock.Now()
	n, err := uc.store.CountResetsSince(ctx, email, now.Add(-uc.window))
	if err != nil {
		return err
	}
	if n >= uc.maxResets {
		return ErrRateLimited
	}
	u, found, err := uc.store.GetUserByEmail(ctx, email)
	if err != nil {
		return err
	}
	if !found {
		return nil
	}
	code, err := idgen.NewCode(6)
	if err != nil {
		return err
	}
	r := Reset{ID: idgen.UUIDGen{}.NewID(), UserID: u.ID, Email: email,
		Token: code, ExpiresAt: now.Add(uc.resetT)}
	if err := uc.store.CreateReset(ctx, r); err != nil {
		return err
	}
	return uc.enqueue(ctx, u, "reset_password", code)
}

// Validate checks a code without consuming it. Like Reset, wrong codes bump
// the attempt counter; past maxCodeAttempts the code reads as invalid.
func (uc *UseCase) Validate(ctx context.Context, email, code string) error {
	_, err := uc.check(ctx, email, code)
	return err
}

// Reset consumes the code atomically, sets the new password, auto-verifies
// the account and (optionally) revokes every active session.
func (uc *UseCase) Reset(ctx context.Context, email, code, newPassword string) error {
	email = strings.ToLower(strings.TrimSpace(email))
	if len(newPassword) < 8 {
		return ErrValidation
	}
	r, err := uc.check(ctx, email, code)
	if err != nil {
		return err
	}
	u, found, err := uc.store.GetUserByEmail(ctx, email)
	if err != nil {
		return err
	}
	if !found {
		return ErrCodeInvalid
	}
	hash, err := authx.HashPassword(newPassword, uc.pepper)
	if err != nil {
		return err
	}
	consumed, err := uc.store.MarkResetUsed(ctx, r.ID, r.Token)
	if err != nil {
		return err
	}
	if !consumed {
		return ErrCodeUsed // a racing request consumed the code first
	}
	if err := uc.store.SetPasswordAndVerify(ctx, u.ID, hash); err != nil {
		return err
	}
	if uc.logOutAllOnReset {
		if err := uc.store.RevokeAllSessions(ctx, u.ID); err != nil {
			return err
		}
	}
	return uc.enqueue(ctx, u, "password_changed", "")
}

// Change verifies the current password, then rotates it and revokes every
// OTHER active session — the caller's own session survives via jti.
func (uc *UseCase) Change(ctx context.Context, userID, sessionID, current, newPassword string) error {
	if len(newPassword) < 8 {
		return ErrValidation
	}
	u, found, err := uc.store.GetUserByID(ctx, userID)
	if err != nil {
		return err
	}
	if !found || authx.CheckPassword(u.Password, current, uc.pepper) != nil {
		return ErrPasswordWrong
	}
	hash, err := authx.HashPassword(newPassword, uc.pepper)
	if err != nil {
		return err
	}
	if err := uc.store.SetPassword(ctx, u.ID, hash); err != nil {
		return err
	}
	if err := uc.store.RevokeOtherSessions(ctx, u.ID, sessionID); err != nil {
		return err
	}
	return uc.enqueue(ctx, u, "password_changed", "")
}

// check loads the latest reset row for the email, validates state and (on a
// wrong code) bumps the attempt counter. Constant-time token compare.
func (uc *UseCase) check(ctx context.Context, email, code string) (Reset, error) {
	r, ok, err := uc.store.LatestReset(ctx, email)
	if err != nil {
		return Reset{}, err
	}
	if !ok {
		return Reset{}, ErrCodeInvalid
	}
	switch {
	case r.Used:
		return Reset{}, ErrCodeUsed
	case uc.clock.Now().After(r.ExpiresAt):
		return Reset{}, ErrCodeExpired
	case r.Attempts >= maxCodeAttempts:
		return Reset{}, ErrCodeInvalid
	case subtle.ConstantTimeCompare([]byte(r.Token), []byte(code)) != 1:
		if err := uc.store.BumpReset(ctx, r.ID); err != nil {
			return Reset{}, err
		}
		return Reset{}, ErrCodeInvalid
	}
	return r, nil
}

// enqueue renders (per user language) and queues one email job.
func (uc *UseCase) enqueue(ctx context.Context, u userRow, template, code string) error {
	locale := u.Language
	if locale == "" {
		locale = "en"
	}
	return uc.emails.Enqueue(ctx, email.Message{
		To:          u.Email,
		TemplateKey: template,
		Vars:        map[string]any{"first_name": u.FirstName, "code": code, "locale": locale},
	})
}
