package authsignup

import (
	"net/http"

	"github.com/<scope>/<app>/internal/gen/api"
	"github.com/<scope>/<app>/internal/platform/httpx"
)

type handlers struct {
	uc *UseCase
}

// accepted is the uniform 200 body for verify/resend acknowledgements.
func accepted(msg string) api.MessageResponse {
	return api.MessageResponse{Message: &msg}
}

// signup: 201 on first signup or re-issued code (never 200/202); an unknown
// user_type is a 422 from the use-case validation.
func (h handlers) signup(w http.ResponseWriter, r *http.Request) {
	var req api.SignupRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.Signup(r.Context(), SignupInput{
		Email:     string(req.Email),
		FirstName: deref(req.FirstName),
		LastName:  deref(req.LastName),
		Password:  req.Password,
		Language:  deref(req.Language),
		UserType:  userTypeString(req.UserType),
	}); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, accepted("signup successful"))
}

// userTypeString dereferences the optional user_type enum; absent ("") lets
// the use case apply the COMPANY default.
func userTypeString(p *api.SignupRequestUserType) string {
	if p == nil {
		return ""
	}
	return string(*p)
}

// verifyEmail consumes a 6-digit code: 200, or 400 with the code-status error.
func (h handlers) verifyEmail(w http.ResponseWriter, r *http.Request) {
	var req api.VerifyEmailRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.VerifyEmail(r.Context(), string(req.Email), req.Code); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, accepted("email verified"))
}

// resend: always 200, even when the account does not exist (no leak).
func (h handlers) resend(w http.ResponseWriter, r *http.Request) {
	var req api.EmailRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.ResendVerification(r.Context(), string(req.Email)); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, accepted("verification code sent"))
}

func deref(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
