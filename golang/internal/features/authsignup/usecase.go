package authsignup

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

// UseCase implements the signup / verify / resend flows.
type UseCase struct {
	store   Store
	emails  EmailEnqueuer
	clock   clockx.Clock
	pepper  string
	verifyT time.Duration
}

// New wires the use-case; a nil clock falls back to the system clock.
func New(store Store, emails EmailEnqueuer, clock clockx.Clock, pepper string, verifyT time.Duration) *UseCase {
	if clock == nil {
		clock = clockx.System{}
	}
	return &UseCase{store: store, emails: emails, clock: clock, pepper: pepper, verifyT: verifyT}
}

// SignupInput carries the decoded request. UserType is optional ("", ADMIN,
// COMPANY, USER — case-insensitive); blank resolves to COMPANY.
type SignupInput struct {
	Email, FirstName, LastName, Password, Language, UserType string
}

// Signup normalizes the email, creates the user unverified (with the optional
// user_type) and enqueues the verification email; for an existing unverified
// user it re-issues a code instead (idempotent). New users also get their
// 1:1 profile row: COMPANY → companies (company_name = first + last), USER →
// users_profile (full_name = first + last), ADMIN → none.
func (uc *UseCase) Signup(ctx context.Context, in SignupInput) error {
	in.Email = strings.ToLower(strings.TrimSpace(in.Email))
	if in.Email == "" || len(in.Password) < 8 {
		return ErrValidation
	}
	userType, ok := NormalizeUserType(in.UserType)
	if !ok {
		return ErrValidation
	}
	now := uc.clock.Now()
	existing, found, err := uc.store.GetUserByEmail(ctx, in.Email)
	if err != nil {
		return err
	}
	if found && existing.IsVerified {
		return ErrEmailTakenActive
	}
	var userID, lang string
	if found {
		userID = existing.ID
		lang = defaultLang(existing.Language)
	} else {
		hash, err := authx.HashPassword(in.Password, uc.pepper)
		if err != nil {
			return err
		}
		userID = idgen.UUIDGen{}.NewID()
		lang = defaultLang(in.Language)
		row := UserRow{
			ID: userID, Email: in.Email,
			FirstName: in.FirstName, LastName: in.LastName,
			PasswordHash: hash,
			Role:         "USER", UserType: userType, Language: lang,
			IsActive: true, IsVerified: false,
		}
		if err := uc.store.CreateUser(ctx, row); err != nil {
			return err
		}
		if err := uc.createProfile(ctx, userID, userType, in.FirstName, in.LastName); err != nil {
			return err
		}
	}
	// Re-issue regardless of path: one pending code, always freshly sent.
	code, err := idgen.NewCode(6)
	if err != nil {
		return err
	}
	v := Verification{ID: idgen.UUIDGen{}.NewID(), UserID: userID, Token: code, ExpiresAt: now.Add(uc.verifyT)}
	if err := uc.store.CreateVerification(ctx, v); err != nil {
		return err
	}
	return uc.enqueueVerification(ctx, existing, in.Email, in.FirstName, lang, code)
}

// VerifyEmail consumes the 6-digit code and flips is_verified. Wrong codes
// bump the attempt counter; past maxCodeAttempts the code reads as invalid,
// so 24h of guessing cannot exhaust the 6-digit space.
func (uc *UseCase) VerifyEmail(ctx context.Context, emailAddr, code string) error {
	emailAddr = strings.ToLower(strings.TrimSpace(emailAddr))
	if code == "" {
		return ErrCodeInvalid
	}
	user, found, err := uc.store.GetUserByEmail(ctx, emailAddr)
	if err != nil {
		return err
	}
	if !found {
		return ErrCodeInvalid
	}
	v, ok, err := uc.store.LatestVerification(ctx, user.ID)
	if err != nil {
		return err
	}
	if !ok {
		return ErrCodeInvalid
	}
	switch {
	case v.Used:
		return ErrCodeUsed
	case uc.clock.Now().After(v.ExpiresAt):
		return ErrCodeExpired
	case v.Attempts >= maxCodeAttempts:
		return ErrCodeInvalid
	case subtle.ConstantTimeCompare([]byte(v.Token), []byte(code)) != 1:
		if err := uc.store.BumpVerification(ctx, v.ID); err != nil {
			return err
		}
		return ErrCodeInvalid
	}
	consumed, err := uc.store.UseVerification(ctx, v.ID, v.Token)
	if err != nil {
		return err
	}
	if !consumed {
		return ErrCodeUsed // a racing request consumed the code first
	}
	return uc.store.SetEmailVerified(ctx, user.ID)
}

// ResendVerification creates a fresh code for an unverified user; unknown
// and already-verified emails answer the same way (no existence leak).
func (uc *UseCase) ResendVerification(ctx context.Context, emailAddr string) error {
	emailAddr = strings.ToLower(strings.TrimSpace(emailAddr))
	user, found, err := uc.store.GetUserByEmail(ctx, emailAddr)
	if err != nil {
		return err
	}
	if !found || user.IsVerified {
		return nil
	}
	code, err := idgen.NewCode(6)
	if err != nil {
		return err
	}
	v := Verification{ID: idgen.UUIDGen{}.NewID(), UserID: user.ID, Token: code, ExpiresAt: uc.clock.Now().Add(uc.verifyT)}
	if err := uc.store.CreateVerification(ctx, v); err != nil {
		return err
	}
	return uc.enqueueVerification(ctx, user, user.Email, user.FirstName, user.Language, code)
}

func (uc *UseCase) enqueueVerification(ctx context.Context, row UserRow, emailAddr, firstName, lang, code string) error {
	locale := lang
	if locale == "" {
		locale = "en"
	}
	return uc.emails.Enqueue(ctx, email.Message{
		To:          emailAddr,
		TemplateKey: "verify_email",
		Vars:        map[string]any{"first_name": firstName, "code": code, "locale": locale},
	})
}

// createProfile seeds the 1:1 profile row for the user's type; ADMIN needs
// none. ponytail: runs after CreateUser on a plain pool (no tx) — a failed
// insert leaves an unverified user without its profile and re-signup never
// re-seeds it; wrap the insert path in a store-level tx only if that window
// actually bites.
func (uc *UseCase) createProfile(ctx context.Context, userID, userType, firstName, lastName string) error {
	name := strings.TrimSpace(firstName + " " + lastName)
	switch userType {
	case UserTypeUser:
		return uc.store.CreateUserProfile(ctx, userID, name)
	case UserTypeCompany:
		return uc.store.CreateCompanyProfile(ctx, userID, name)
	default: // ADMIN: no profile table by design
		return nil
	}
}

// defaultLang resolves the user language; empty => "en".
func defaultLang(lang string) string {
	if lang == "" {
		return "en"
	}
	return lang
}
