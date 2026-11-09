package authpassword

import (
	"net/http"

	"github.com/<scope>/<app>/internal/gen/api"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/httpx"
)

type handlers struct {
	uc *UseCase
}

// ok is the uniform 200 acknowledgement.
func ok(msg string) api.MessageResponse { return api.MessageResponse{Message: &msg} }

func (h handlers) forgot(w http.ResponseWriter, r *http.Request) {
	var req api.EmailRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.Forgot(r.Context(), string(req.Email)); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, ok("if the account exists, a reset code was sent"))
}

func (h handlers) validate(w http.ResponseWriter, r *http.Request) {
	var req api.ValidateCodeRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.Validate(r.Context(), string(req.Email), req.Code); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, ok("code is valid"))
}

func (h handlers) reset(w http.ResponseWriter, r *http.Request) {
	var req api.ResetPasswordRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	if err := h.uc.Reset(r.Context(), string(req.Email), req.Code, req.NewPassword); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, ok("password updated"))
}

// change: bearer-protected; jti (== sessions.id) is what survives revocation.
func (h handlers) change(w http.ResponseWriter, r *http.Request) {
	var req api.ChangePasswordRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	id, _ := authx.IdentityFrom(r.Context())
	if err := h.uc.Change(r.Context(), id.UserID, id.JTI, req.CurrentPassword, req.NewPassword); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, ok("password updated"))
}
