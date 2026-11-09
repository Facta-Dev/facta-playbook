package adminusers

import (
	"net/http"

	"github.com/google/uuid"
	oapi_types "github.com/oapi-codegen/runtime/types"

	"github.com/<scope>/<app>/internal/gen/api"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/shared/apperr"
)

type handlers struct {
	uc *UseCase
}

// adminModel maps a slice Row onto the generated API model.
func adminModel(r Row) api.AdminUser {
	res := api.AdminUser{
		Email:      emailPtr(r.Email),
		Id:         uuidPtr(r.ID),
		FirstName:  &r.FirstName,
		LastName:   &r.LastName,
		Role:       strPtr[api.AdminUserRole](r.Role),
		UserType:   strPtr[api.AdminUserUserType](r.UserType),
		IsActive:   &r.IsActive,
		IsBlocked:  &r.IsBlocked,
		IsVerified: &r.IsVerified,
		Language:   &r.Language,
		CreatedAt:  &r.CreatedAt,
	}
	if !r.LastLogin.IsZero() {
		res.LastLogin = &r.LastLogin
	}
	return res
}

func (h handlers) list(w http.ResponseWriter, r *http.Request) {
	limit, offset, err := httpx.QueryPage(r)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	rows, total, err := h.uc.List(r.Context(), int(limit), int(offset))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	items := make([]api.AdminUser, 0, len(rows))
	for i := range rows {
		items = append(items, adminModel(rows[i]))
	}
	totalInt := total
	httpx.WriteJSON(w, http.StatusOK, api.AdminUserList{Items: &items, Total: &totalInt})
}

func (h handlers) get(w http.ResponseWriter, r *http.Request) {
	row, err := h.uc.Get(r.Context(), r.PathValue("id"))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, adminModel(row))
}

// update applies the nullable patch and returns the fresh row.
func (h handlers) update(w http.ResponseWriter, r *http.Request) {
	var req api.AdminUpdateUser
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request"))
		return
	}
	actor, _ := authx.IdentityFrom(r.Context())
	row, err := h.uc.Update(r.Context(), actor.UserID, actor.Role, r.PathValue("id"),
		httpx.RequestIP(r), r.UserAgent(), Patch{
			IsActive:  req.IsActive,
			IsBlocked: req.IsBlocked,
			Role:      roleString(req.Role),
		})
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, adminModel(row))
}

// upgradeToAdmin is the unauthenticated bootstrap route: the
// <APP>_ALLOWED_ADMIN_EMAILS whitelist is the only gate, so no RequireAuth
// here (reference upgrade_to_admin parity).
func (h handlers) upgradeToAdmin(w http.ResponseWriter, r *http.Request) {
	var req api.EmailRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request"))
		return
	}
	res, err := h.uc.UpgradeToAdmin(r.Context(), string(req.Email))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, api.AuthUpgradeResponse{
		Email:    emailPtr(res.Email),
		Message:  res.Message,
		Upgraded: res.Upgraded,
	})
}

func roleString(p *api.AdminUpdateUserRole) string {
	if p == nil {
		return ""
	}
	return string(*p)
}

func emailPtr(e string) *oapi_types.Email { v := oapi_types.Email(e); return &v }
func strPtr[E ~string](s string) *E       { v := E(s); return &v }

// uuidPtr parses an id into the generated pointer model; nil on bad input.
func uuidPtr(s string) *oapi_types.UUID {
	u, err := uuid.Parse(s)
	if err != nil {
		return nil
	}
	return &u
}
