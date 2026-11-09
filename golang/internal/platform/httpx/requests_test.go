package httpx

import (
	"strings"
	"testing"

	oapi_types "github.com/oapi-codegen/runtime/types"

	"github.com/<scope>/<app>/internal/gen/api"
)

// TestRequestConstraintMirror pins the struct-level registrations to the
// constraints the OpenAPI schemas declare: a violating request fails Decode's
// validation with a 422 and a satisfying one passes. Empty emails fail too
// (format: email on a required field).
func TestRequestConstraintMirror(t *testing.T) {
	t.Parallel()
	name := strings.Repeat("n", 101) // > openapi first_name maxLength: 100

	t.Run("email-format", func(t *testing.T) {
		t.Parallel()
		for _, bad := range []string{"", "no-at-sign", "a@ex"} {
			if err := validate.Struct(api.AuthLoginRequest{Email: apiEmail(bad), Password: "x"}); err == nil {
				t.Fatalf("email %q must fail", bad)
			}
		}
		if err := validate.Struct(api.AuthLoginRequest{Email: apiEmail("ada@ex.com"), Password: "x"}); err != nil {
			t.Fatalf("valid email must pass: %v", err)
		}
	})

	t.Run("signup-declared-constraints", func(t *testing.T) {
		t.Parallel()
		long := name
		req := api.SignupRequest{Email: apiEmail("ada@ex.com"), Password: strings.Repeat("p", 8),
			FirstName: &long}
		if err := validate.Struct(req); err == nil {
			t.Fatal("101-char first_name must fail")
		}
		short := strings.Repeat("n", 100)
		req.FirstName = &short
		req.LastName = &short
		lang := strings.Repeat("l", 11) // > language maxLength: 10
		req.Language = &lang
		if err := validate.Struct(req); err == nil {
			t.Fatal("11-char language must fail")
		}
	})

	t.Run("password-and-code", func(t *testing.T) {
		t.Parallel()
		if err := validate.Struct(api.SignupRequest{Email: apiEmail("a@ex.com"), Password: "short"}); err == nil {
			t.Fatal("7-char password must fail")
		}
		if err := validate.Struct(api.VerifyEmailRequest{Email: apiEmail("a@ex.com"), Code: "12a45"}); err == nil {
			t.Fatal("non-digit code must fail")
		}
		if err := validate.Struct(api.ResetPasswordRequest{Email: apiEmail("a@ex.com"), Code: "123456",
			NewPassword: "short"}); err == nil {
			t.Fatal("short new_password must fail")
		}
		if err := validate.Struct(api.ChangePasswordRequest{NewPassword: strings.Repeat("p", 8)}); err != nil {
			t.Fatalf("valid change must pass: %v", err)
		}
	})
}

// apiEmail wraps a plain string in the generated email type.
func apiEmail(s string) oapi_types.Email {
	return oapi_types.Email(s)
}
