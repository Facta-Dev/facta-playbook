package authpassword

import (
	"errors"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

// newUC wires the use-case with the injected clock; expiry 30m, window 5m.
func newUC(store Store, emails EmailEnqueuer, now time.Time, logOutAll bool) *UseCase {
	return New(store, emails, clockx.Fixed(now), "pepper-32-bytes-long-enough!!!",
		30*time.Minute, 5*time.Minute, 3, logOutAll)
}

func TestForgotRateLimit(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store, emails := newFakeStore(now), &fakeEmails{}
	uc := newUC(store, emails, now, true)
	hash, err := authx.HashPassword("pw-not-used", "pepper-32-bytes-long-enough!!!")
	if err != nil {
		t.Fatalf("hash: %v", err)
	}
	_ = hash

	for i := 0; i < 3; i++ {
		if err := uc.Forgot(t.Context(), "a@ex.com"); err != nil {
			t.Fatalf("forgot %d: %v", i, err)
		}
	}
	err = uc.Forgot(t.Context(), "a@ex.com")
	var ae *apperr.AppError
	if !errors.As(err, &ae) || ae.HTTP != 429 || ae.Code != apperr.AuthRateLimit {
		t.Fatalf("want 429 %s, got %v", apperr.AuthRateLimit, err)
	}
	if len(emails.sent) != 3 {
		t.Fatalf("queued %d emails, want 3", len(emails.sent))
	}

	// A different email is still allowed (limit is per email).
	if err := uc.Forgot(t.Context(), "other@ex.com"); err != nil {
		t.Fatalf("other email: %v", err)
	}
}

// forgot for an unknown email stays silent (200 semantics upstream).
func TestForgotUnknownIsSilent(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(time.Unix(1700, 0)), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0), true)
	if err := uc.Forgot(t.Context(), "ghost@ex.com"); err != nil {
		t.Fatalf("forgot: %v", err)
	}
	if len(emails.sent) != 0 {
		t.Fatalf("no email should be queued, got %d", len(emails.sent))
	}
}

func TestValidateAndStates(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store, emails := newFakeStore(now), &fakeEmails{}
	uc := newUC(store, emails, now, true)
	if err := uc.Forgot(t.Context(), "a@ex.com"); err != nil {
		t.Fatalf("forgot: %v", err)
	}
	var code string
	for _, r := range store.resets {
		if r.Email == "a@ex.com" {
			code = r.Token
		}
	}

	if err := uc.Validate(t.Context(), "a@ex.com", "000000"); !errors.Is(err, ErrCodeInvalid) {
		t.Fatalf("wrong code: %v", err)
	}
	if err := uc.Validate(t.Context(), "a@ex.com", code); err != nil {
		t.Fatalf("valid code: %v", err)
	}
	// Expiry: advance the clock past the 30m lifetime.
	later := newUC(store, emails, now.Add(31*time.Minute), true)
	if err := later.Validate(t.Context(), "a@ex.com", code); !errors.Is(err, ErrCodeExpired) {
		t.Fatalf("expired: %v", err)
	}
}

func TestResetAutoVerifiesAndRevokes(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store, emails := newFakeStore(now), &fakeEmails{}
	uc := newUC(store, emails, now, true)
	if err := uc.Forgot(t.Context(), "a@ex.com"); err != nil {
		t.Fatalf("forgot: %v", err)
	}
	var code string
	for _, r := range store.resets {
		if r.Email == "a@ex.com" {
			code = r.Token
		}
	}
	store.sessions["s-old"] = true
	store.sessions["s-keep"] = true

	if err := uc.Reset(t.Context(), "a@ex.com", code, "new-password-99"); err != nil {
		t.Fatalf("reset: %v", err)
	}
	u := store.users["u-1"]
	if !u.IsVerified {
		t.Fatal("reset must auto-verify the account")
	}
	if authx.CheckPassword(u.Password, "new-password-99", "pepper-32-bytes-long-enough!!!") != nil {
		t.Fatal("password not rotated")
	}
	if store.sessions["s-old"] || store.sessions["s-keep"] {
		t.Fatal("sessions not revoked on reset")
	}
	if len(emails.sent) < 2 || emails.sent[len(emails.sent)-1].TemplateKey != "password_changed" {
		t.Fatalf("bad email jobs: %+v", emails.sent)
	}
	if err := uc.Reset(t.Context(), "a@ex.com", code, "another-password"); !errors.Is(err, ErrCodeUsed) {
		t.Fatalf("replay: %v", err)
	}
}

func TestResetKeepsSessionsWhenConfigured(t *testing.T) {
	t.Parallel()
	now := time.Unix(1700, 0)
	store, emails := newFakeStore(now), &fakeEmails{}
	uc := newUC(store, emails, now, false)
	if err := uc.Forgot(t.Context(), "a@ex.com"); err != nil {
		t.Fatalf("forgot: %v", err)
	}
	var code string
	for _, r := range store.resets {
		if r.Email == "a@ex.com" {
			code = r.Token
		}
	}
	store.sessions["s-active"] = true
	if err := uc.Reset(t.Context(), "a@ex.com", code, "new-password-99"); err != nil {
		t.Fatalf("reset: %v", err)
	}
	if !store.sessions["s-active"] {
		t.Fatal("sessions must survive when logout-all is off")
	}
}

func TestChangeKeepsCurrentSession(t *testing.T) {
	t.Parallel()
	store := newFakeStore(time.Unix(1700, 0))
	uc := newUC(store, &fakeEmails{}, time.Unix(1700, 0), true)
	u := store.users["u-1"]
	u.Password = hashFor(t, "current-pw-1", "pepper-32-bytes-long-enough!!!")
	store.users["u-1"] = u
	store.sessions["mine"] = true
	store.sessions["other"] = true

	if err := uc.Change(t.Context(), "u-1", "mine", "wrong", "new-password-99"); !errors.Is(err, ErrPasswordWrong) {
		t.Fatalf("wrong current: %v", err)
	}
	if err := uc.Change(t.Context(), "u-1", "mine", "current-pw-1", "new-password-99"); err != nil {
		t.Fatalf("change: %v", err)
	}
	u = store.users["u-1"]
	if authx.CheckPassword(u.Password, "new-password-99", "pepper-32-bytes-long-enough!!!") != nil {
		t.Fatal("password not rotated")
	}
	if !store.sessions["mine"] {
		t.Fatal("current session must survive (jti kept)")
	}
	if store.sessions["other"] {
		t.Fatal("other sessions must be revoked")
	}
}

// hashFor is a small local helper (the session slice has its own).
func hashFor(t *testing.T, password, pepper string) string {
	t.Helper()
	h, err := authx.HashPassword(password, pepper)
	if err != nil {
		t.Fatalf("hash: %v", err)
	}
	return h
}
