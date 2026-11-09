package authsignup

import (
	"context"
	"errors"
	"strings"
	"testing"
	"time"

	"github.com/<scope>/<app>/internal/platform/email"
	"github.com/<scope>/<app>/internal/shared/apperr"
	"github.com/<scope>/<app>/internal/shared/clockx"
)

type fakeStore struct {
	users       map[string]UserRow
	verifs      []Verification
	usedIDs     map[string]bool
	verifiedIDs map[string]bool
	companies   map[string]string // userID -> company_name
	profiles    map[string]string // userID -> full_name
	failNext    error
}

func newFakeStore() *fakeStore {
	return &fakeStore{
		users: map[string]UserRow{}, usedIDs: map[string]bool{}, verifiedIDs: map[string]bool{},
		companies: map[string]string{}, profiles: map[string]string{},
	}
}

func (f *fakeStore) GetUserByEmail(_ context.Context, email string) (UserRow, bool, error) {
	if f.failNext != nil {
		return UserRow{}, false, f.failNext
	}
	u, ok := f.users[email]
	return u, ok, nil
}

func (f *fakeStore) CreateUser(_ context.Context, row UserRow) error {
	if f.failNext != nil {
		return f.failNext
	}
	f.users[row.Email] = row
	return nil
}

func (f *fakeStore) CreateVerification(_ context.Context, v Verification) error {
	f.verifs = append(f.verifs, v)
	return nil
}

// LatestVerification returns the last created code for the user.
func (f *fakeStore) LatestVerification(_ context.Context, userID string) (Verification, bool, error) {
	for i := len(f.verifs) - 1; i >= 0; i-- {
		if f.verifs[i].UserID == userID {
			v := f.verifs[i]
			v.Used = f.usedIDs[v.ID]
			return v, true, nil
		}
	}
	return Verification{}, false, nil
}

func (f *fakeStore) UseVerification(_ context.Context, vID, token string) (bool, error) {
	for i := range f.verifs {
		if f.verifs[i].ID == vID {
			if f.verifs[i].Used || f.verifs[i].Token != token {
				return false, nil
			}
			f.usedIDs[vID] = true
			return true, nil
		}
	}
	return false, nil
}

func (f *fakeStore) BumpVerification(_ context.Context, vID string) error {
	for i := range f.verifs {
		if f.verifs[i].ID == vID {
			f.verifs[i].Attempts++
		}
	}
	return nil
}

func (f *fakeStore) SetEmailVerified(_ context.Context, userID string) error {
	f.verifiedIDs[userID] = true
	for email, u := range f.users {
		if u.ID == userID {
			u.IsVerified = true
			f.users[email] = u
		}
	}
	return nil
}

func (f *fakeStore) CreateCompanyProfile(_ context.Context, userID, companyName string) error {
	if f.failNext != nil {
		return f.failNext
	}
	f.companies[userID] = companyName
	return nil
}

func (f *fakeStore) CreateUserProfile(_ context.Context, userID, fullName string) error {
	if f.failNext != nil {
		return f.failNext
	}
	f.profiles[userID] = fullName
	return nil
}

type fakeEmails struct{ sent []email.Message }

func (f *fakeEmails) Enqueue(_ context.Context, ms email.Message) error {
	f.sent = append(f.sent, ms)
	return nil
}

func newUC(store Store, emails EmailEnqueuer, now time.Time) *UseCase {
	return New(store, emails, clockx.Fixed(now), "pepper-secret-32-bytes-long!", time.Hour)
}

func TestSignupCreatesUnverifiedAndQueuesCode(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))

	if err := uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password", FirstName: "Ada"}); err != nil {
		t.Fatalf("signup: %v", err)
	}
	u := store.users["a@ex.com"]
	if u.ID == "" || u.IsVerified || !u.IsActive || u.Role != "USER" || u.PasswordHash == "" {
		t.Fatalf("bad persisted row: %+v", u)
	}
	if len(emails.sent) != 1 || emails.sent[0].TemplateKey != "verify_email" {
		t.Fatalf("bad email jobs: %+v", emails.sent)
	}
	if code := emails.sent[0].Vars["code"]; code == "" {
		t.Fatal("missing code var")
	}
}

func TestSignupUserTypeAndProfiles(t *testing.T) {
	t.Parallel()
	tests := []struct {
		name     string
		userType string // "" exercises the optional/default path
		company  bool   // expects a companies row
		personal bool   // expects a users_profile row
	}{
		{name: "default is company", userType: "", company: true},
		{name: "explicit company", userType: "COMPANY", company: true},
		{name: "lowercase user", userType: "user", personal: true},
		{name: "admin has no profile", userType: "ADMIN"},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()
			store, emails := newFakeStore(), &fakeEmails{}
			uc := newUC(store, emails, time.Unix(1700, 0))

			if err := uc.Signup(t.Context(), SignupInput{
				Email: "a@ex.com", Password: "long-password",
				FirstName: "Ada", LastName: "Lovelace", UserType: tt.userType,
			}); err != nil {
				t.Fatalf("signup: %v", err)
			}
			u := store.users["a@ex.com"]
			wantType := strings.ToUpper(tt.userType)
			if wantType == "" {
				wantType = UserTypeCompany
			}
			if u.UserType != wantType {
				t.Fatalf("user_type stored: %s, want %s", u.UserType, wantType)
			}
			if got := store.companies[u.ID]; (got != "") != tt.company || (got != "" && got != "Ada Lovelace") {
				t.Fatalf("company profile: got %q, want row=%v named %q", got, tt.company, "Ada Lovelace")
			}
			if got := store.profiles[u.ID]; (got != "") != tt.personal || (got != "" && got != "Ada Lovelace") {
				t.Fatalf("personal profile: got %q, want row=%v named %q", got, tt.personal, "Ada Lovelace")
			}
			// role stays the plain USER role; user_type is a separate axis.
			if u.Role != "USER" {
				t.Fatalf("role must not follow user_type, got %s", u.Role)
			}
		})
	}
}

func TestSignupUnknownUserTypeRejected(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))

	err := uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password", UserType: "LAWFIRM"})
	if !errors.Is(err, ErrValidation) {
		t.Fatalf("want validation error, got %v", err)
	}
	if len(store.users) != 0 || len(store.companies) != 0 || len(store.profiles) != 0 {
		t.Fatalf("rejected signup must create nothing: %+v", store)
	}
	if len(emails.sent) != 0 {
		t.Fatal("rejected signup must not queue email")
	}
}

func TestSignupResignupDoesNotReseedProfile(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))
	_ = uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password", FirstName: "Ada"})
	first := len(store.companies)
	if err := uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password", FirstName: "Ada", LastName: "Lovelace"}); err != nil {
		t.Fatalf("re-signup must succeed: %v", err)
	}
	if len(store.companies) != first {
		t.Fatalf("re-signup re-seeded the profile (%d)", len(store.companies)-first)
	}
}

func TestSignupIdempotentReissue(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))
	_ = uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"})
	first := len(emails.sent)
	if err := uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"}); err != nil {
		t.Fatalf("re-signup must succeed: %v", err)
	}
	if len(store.users) != 1 {
		t.Fatalf("duplicate user created")
	}
	if len(emails.sent) != first+1 {
		t.Fatalf("expected one more code, got %d", len(emails.sent)-first)
	}
}

func TestSignupVerifiedConflict(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))
	_ = uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"})
	store.SetEmailVerified(t.Context(), store.users["a@ex.com"].ID)

	err := uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"})
	var ae *apperr.AppError
	if !errors.As(err, &ae) || ae.Code != apperr.EmailTakenAct {
		t.Fatalf("want 409 %s, got %v", apperr.EmailTakenAct, err)
	}
}

func TestVerifyEmailStates(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))
	_ = uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"})
	u := store.users["a@ex.com"]
	code, ok, _ := store.LatestVerification(t.Context(), u.ID)
	if !ok {
		t.Fatal("no verification stored")
	}

	if err := uc.VerifyEmail(t.Context(), "a@ex.com", code.Token[:3]+"000"); !errors.Is(err, ErrCodeInvalid) {
		t.Fatalf("wrong code: %v", err)
	}
	if err := uc.VerifyEmail(t.Context(), "a@ex.com", code.Token); err != nil {
		t.Fatalf("valid code: %v", err)
	}
	if !store.verifiedIDs[u.ID] {
		t.Fatal("is_verified not set")
	}
	if err := uc.VerifyEmail(t.Context(), "a@ex.com", code.Token); !errors.Is(err, ErrCodeUsed) {
		t.Fatalf("replay: %v", err)
	}
}

func TestVerifyEmailExpired(t *testing.T) {
	t.Parallel()
	store, emails := newFakeStore(), &fakeEmails{}
	uc := newUC(store, emails, time.Unix(1700, 0))
	_ = uc.Signup(t.Context(), SignupInput{Email: "a@ex.com", Password: "long-password"})
	u := store.users["a@ex.com"]
	code, ok, _ := store.LatestVerification(t.Context(), u.ID)
	if !ok {
		t.Fatal("no verification stored")
	}
	// The expiry check runs against a clock 2h later; verifyT is 1h.
	later := newUC(store, emails, time.Unix(1700, 0).Add(2*time.Hour))
	if err := later.VerifyEmail(t.Context(), "a@ex.com", code.Token); !errors.Is(err, ErrCodeExpired) {
		t.Fatalf("expired: %v", err)
	}
}
