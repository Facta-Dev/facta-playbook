// The OpenAPI request schemas declare format/minLength/maxLength/pattern
// constraints that models-only generation drops (the generated fields carry
// no validate tags). The struct-level registrations below enforce them at the
// Decode edge, mirroring specs/openapi.yaml exactly — nothing here invents a
// constraint.
package httpx

import (
	"regexp"

	"github.com/go-playground/validator/v10"
	oapi_types "github.com/oapi-codegen/runtime/types"

	"github.com/<scope>/<app>/internal/gen/api"
)

// Constants mirror the schema declarations (openapi minLength/maxLength).
const (
	passwordMin = 8   // password / new_password minLength: 8
	nameMax     = 100 // first_name/last_name maxLength
	languageMax = 10  // language maxLength
	phoneMax    = 20  // phone_number/postal_code maxLength
	pictureMax  = 255 // profile_picture maxLength
)

// codeDigitRE is the code pattern declared in openapi (`^[0-9]{6}$`); the
// companion maxLength: 8 cannot fire once the pattern pins six digits.
var codeDigitRE = regexp.MustCompile(`^[0-9]{6}$`)

// rep reports one violated constraint: the field value, its Go-style field
// name and the failing-constraint tag.
type rep func(field any, name, tag string)

// register wires one struct-level check for one generated request type —
// the same validator instance Decode already runs.
func register[T any](sample T, check func(T, rep)) {
	validate.RegisterStructValidation(func(sl validator.StructLevel) {
		r := sl.Current().Interface().(T)
		check(r, func(field any, name, tag string) {
			sl.ReportError(field, name, name, tag, "")
		})
	}, sample)
}

// emailRep guards format:email, declared on every email-bearing schema.
func emailRep(e oapi_types.Email, r rep) {
	if validate.Var(string(e), "email") != nil { // an empty string fails too
		r(e, "Email", "email")
	}
}

// codeRep guards the six-digit code pattern (VerifyEmail / ValidateCode / Reset).
func codeRep(s string, r rep) {
	if !codeDigitRE.MatchString(s) {
		r(s, "Code", "code")
	}
}

// passwordRep guards the declared newPassword/password minLength.
func passwordRep(s string, r rep) {
	if len(s) < passwordMin {
		r(s, "Password", "minLength")
	}
}

func init() {
	// format: email — plain email-bearing schemas.
	register(api.AuthLoginRequest{}, func(req api.AuthLoginRequest, r rep) { emailRep(req.Email, r) })
	register(api.EmailRequest{}, func(req api.EmailRequest, r rep) { emailRep(req.Email, r) })
	// google one-tap: the credential is required.
	register(api.GoogleAuthRequest{}, func(req api.GoogleAuthRequest, r rep) {
		if req.Credential == "" {
			r(req.Credential, "Credential", "required")
		}
	})
	// code schemas: email + the six-digit pattern.
	register(api.VerifyEmailRequest{}, func(req api.VerifyEmailRequest, r rep) {
		emailRep(req.Email, r)
		codeRep(req.Code, r)
	})
	register(api.ValidateCodeRequest{}, func(req api.ValidateCodeRequest, r rep) {
		emailRep(req.Email, r)
		codeRep(req.Code, r)
	})
	// reset: email + code + the new password minimum.
	register(api.ResetPasswordRequest{}, func(req api.ResetPasswordRequest, r rep) {
		emailRep(req.Email, r)
		codeRep(req.Code, r)
		passwordRep(req.NewPassword, r)
	})
	// change: the new password minimum (current_password carries no declared
	// constraint).
	register(api.ChangePasswordRequest{}, func(req api.ChangePasswordRequest, r rep) {
		passwordRep(req.NewPassword, r)
	})
	// signup: email + password minimum + optional name/language maxima.
	register(api.SignupRequest{}, func(req api.SignupRequest, r rep) {
		emailRep(req.Email, r)
		passwordRep(req.Password, r)
		if req.FirstName != nil && len(*req.FirstName) > nameMax {
			r(req.FirstName, "FirstName", "maxLength")
		}
		if req.LastName != nil && len(*req.LastName) > nameMax {
			r(req.LastName, "LastName", "maxLength")
		}
		if req.Language != nil && len(*req.Language) > languageMax {
			r(req.Language, "Language", "maxLength")
		}
	})
	// update-me: optional fields, declared maxima only ('' means cleared-to-
	// empty-string; presence is the pointer, not the emptiness).
	register(api.UpdateMeRequest{}, func(req api.UpdateMeRequest, r rep) {
		if req.FirstName != nil && len(*req.FirstName) > nameMax {
			r(req.FirstName, "FirstName", "maxLength")
		}
		if req.LastName != nil && len(*req.LastName) > nameMax {
			r(req.LastName, "LastName", "maxLength")
		}
		if req.PhoneNumber != nil && len(*req.PhoneNumber) > phoneMax {
			r(req.PhoneNumber, "PhoneNumber", "maxLength")
		}
		if req.ProfilePicture != nil && len(*req.ProfilePicture) > pictureMax {
			r(req.ProfilePicture, "ProfilePicture", "maxLength")
		}
		if req.Language != nil && len(*req.Language) > languageMax {
			r(req.Language, "Language", "maxLength")
		}
		if req.PostalCode != nil && len(*req.PostalCode) > phoneMax {
			r(req.PostalCode, "PostalCode", "maxLength")
		}
	})
}
