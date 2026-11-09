package authsession

import (
	"context"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/google/uuid"
	oapi_types "github.com/oapi-codegen/runtime/types"

	"github.com/<scope>/<app>/internal/gen/api"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/shared/apperr"
)

type handlers struct {
	uc           *UseCase
	accessTTL    time.Duration
	cookieName   string // "" disables cookie mode
	cookieSecure bool   // Set-Cookie Secure flag (false only for dev)

}

// setCookie issues the access-token cookie when cookie mode is on.
func (h handlers) setCookie(w http.ResponseWriter, token string) {
	if h.cookieName != "" {
		authx.SetAuthCookie(w, h.cookieName, token, h.cookieSecure)
	}
}

// tokenPair converts a use-case Result into the generated wire shape.
func (h handlers) tokenPair(res Result) api.AuthTokens {
	return api.AuthTokens{
		AccessToken:  res.AccessToken,
		RefreshToken: res.RefreshToken,
		TokenType:    api.Bearer,
		ExpiresIn:    int(h.accessTTL.Seconds()),
	}
}

func (h handlers) login(w http.ResponseWriter, r *http.Request) {
	var req api.AuthLoginRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperrValidation())
		return
	}
	res, err := h.uc.Login(r.Context(), LoginInput{
		Email: strings.ToLower(strings.TrimSpace(string(req.Email))), Password: req.Password,
		IPAddress: httpx.RequestIP(r), UserAgent: r.UserAgent(),
	})
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	h.setCookie(w, res.AccessToken)
	httpx.WriteJSON(w, http.StatusOK, h.tokenPair(res))
}

func (h handlers) refresh(w http.ResponseWriter, r *http.Request) {
	var req api.AuthRefreshRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperrValidation())
		return
	}
	res, err := h.uc.Refresh(r.Context(), req.RefreshToken, httpx.RequestIP(r), r.UserAgent())
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	h.setCookie(w, res.AccessToken)
	httpx.WriteJSON(w, http.StatusOK, h.tokenPair(res))
}

// logout: bearer-protected; revokes the given refresh token (idempotent 200).
func (h handlers) logout(w http.ResponseWriter, r *http.Request) {
	var req api.AuthRefreshRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperrValidation())
		return
	}
	id, _ := authx.IdentityFrom(r.Context())
	if err := h.uc.Logout(r.Context(), id.UserID, req.RefreshToken); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	if h.cookieName != "" {
		authx.ClearAuthCookie(w, h.cookieName, h.cookieSecure)
	}
	msg := "logged out"
	httpx.WriteJSON(w, http.StatusOK, api.MessageResponse{Message: &msg})
}

// me: profile of the authenticated identity.
func (h handlers) me(w http.ResponseWriter, r *http.Request) {
	id, _ := authx.IdentityFrom(r.Context())
	user, err := h.uc.Me(r.Context(), id.UserID)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, meResponse(user))
}

// updateMe: partial profile update (reference PUT /account/me parity); the
// decode already enforces the declared maxima via httpx/requests.
func (h handlers) updateMe(w http.ResponseWriter, r *http.Request) {
	var req api.UpdateMeRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, apperrValidation())
		return
	}
	user, err := h.uc.UpdateMe(r.Context(), actorID(r.Context()), UpdateMeInput{
		FirstName: req.FirstName,
		LastName:  req.LastName,
		Phone:     req.PhoneNumber,
		Picture:   req.ProfilePicture,
		Language:  req.Language,
		Address:   req.Address,
		Postal:    req.PostalCode,
	})
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, meResponse(user))
}

// meResponse maps the slice's user row onto the widened generated model.
func meResponse(u UserRow) api.MeResponse {
	res := api.MeResponse{
		Email:           emailPtr(u.Email),
		FirstName:       &u.FirstName,
		LastName:        &u.LastName,
		Role:            strPtr[api.MeResponseRole](u.Role),
		UserType:        strPtr[api.MeResponseUserType](u.UserType),
		IsActive:        &u.IsActive,
		IsBlocked:       &u.IsBlocked,
		IsVerified:      &u.IsVerified,
		RetryCount:      &u.RetryCount,
		Language:        &u.Language,
		PhoneNumber:     u.Phone,
		ProfilePicture:  u.Picture,
		Subscription:    u.Subscription,
		LastLogin:       u.LastLogin,
		GithubConnected: &u.GithubLinked,
		CreatedAt:       &u.CreatedAt,
	}
	if id, err := uuid.Parse(u.ID); err == nil {
		res.Id = &id
	}
	return res
}

func apperrValidation() error {
	return apperr.E(apperr.CodeUnprocessable, apperr.Validation, "invalid request")
}

// ---------------------------------------------------------------------------
// Session-management handlers (reference /user/auth-session + /admin/auth-session
// parity): the wire shapes are slice-local until openapi.yaml grows the
// SessionItem/Anomaly models.
// ---------------------------------------------------------------------------

// sessionItem is the wire shape of one session row (reference
// SessionItemResponse): the refresh token itself is never exposed.
type sessionItem struct {
	ID           string    `json:"id"`
	UserID       string    `json:"user_id"`
	IPAddress    string    `json:"ip_address,omitempty"`
	UserAgent    string    `json:"user_agent,omitempty"`
	CreatedAt    time.Time `json:"created_at"`
	LastActivity time.Time `json:"last_activity"`
	ExpiresAt    time.Time `json:"expires_at"`
	IsActive     bool      `json:"is_active"`
}

type revokeResponse struct {
	Revoked bool `json:"revoked"`
}

type revokeCountResponse struct {
	RevokedCount int `json:"revoked_count"`
}

type anomalyItem struct {
	Type    string         `json:"anomaly_type"`
	Count   int            `json:"count"`
	Details map[string]any `json:"details"`
}

func sessionItemOf(s SessionRow) sessionItem {
	return sessionItem{
		ID: s.ID, UserID: s.UserID, IPAddress: s.IPAddress, UserAgent: s.UserAgent,
		CreatedAt: s.CreatedAt, LastActivity: s.LastActivity,
		ExpiresAt: s.ExpiresAt, IsActive: s.IsActive,
	}
}

// actorID pulls the identity set by RequireAuth.
func actorID(ctx context.Context) string {
	id, _ := authx.IdentityFrom(ctx)
	return id.UserID
}

// queryBool parses a boolean query param with a default.
func queryBool(r *http.Request, key string, def bool) (bool, error) {
	v := r.URL.Query().Get(key)
	if v == "" {
		return def, nil
	}
	b, err := strconv.ParseBool(v)
	if err != nil {
		return false, apperr.E(apperr.CodeUnprocessable, apperr.Validation, key+" must be a boolean")
	}
	return b, nil
}

// sessions lists the caller's sessions, newest activity first.
func (h handlers) sessions(w http.ResponseWriter, r *http.Request) {
	activeOnly, err := queryBool(r, "active_only", true)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	rows, err := h.uc.Sessions(r.Context(), actorID(r.Context()), activeOnly)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	items := make([]sessionItem, 0, len(rows))
	for _, s := range rows {
		items = append(items, sessionItemOf(s))
	}
	httpx.WriteJSON(w, http.StatusOK, items)
}

// currentSession: the caller's most recent active session; the reference's
// session_id hint param is unused there too. No active session renders null.
func (h handlers) currentSession(w http.ResponseWriter, r *http.Request) {
	row, ok, err := h.uc.CurrentSession(r.Context(), actorID(r.Context()))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	if !ok {
		httpx.WriteJSON(w, http.StatusOK, nil)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, sessionItemOf(row))
}

// revokeCurrent revokes the caller's current session: the access token's jti
// is the session id (see mintFor), so no body is needed.
func (h handlers) revokeCurrent(w http.ResponseWriter, r *http.Request) {
	id, _ := authx.IdentityFrom(r.Context())
	revoked, err := h.uc.RevokeCurrent(r.Context(), id.UserID, id.JTI)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, revokeResponse{Revoked: revoked})
}

// revokeAll deactivates every active session of the caller.
func (h handlers) revokeAll(w http.ResponseWriter, r *http.Request) {
	n, err := h.uc.RevokeAll(r.Context(), actorID(r.Context()))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, revokeCountResponse{RevokedCount: n})
}

// adminSessions lists one target user's sessions.
func (h handlers) adminSessions(w http.ResponseWriter, r *http.Request) {
	activeOnly, err := queryBool(r, "active_only", true)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	rows, err := h.uc.Sessions(r.Context(), r.PathValue("id"), activeOnly)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	items := make([]sessionItem, 0, len(rows))
	for _, s := range rows {
		items = append(items, sessionItemOf(s))
	}
	httpx.WriteJSON(w, http.StatusOK, items)
}

// adminRevokeAll revokes every active session of one target user.
func (h handlers) adminRevokeAll(w http.ResponseWriter, r *http.Request) {
	n, err := h.uc.RevokeAll(r.Context(), r.PathValue("id"))
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, revokeCountResponse{RevokedCount: n})
}

// anomalies renders the admin session report (?window_minutes=, default 60).
func (h handlers) anomalies(w http.ResponseWriter, r *http.Request) {
	window := time.Hour
	if v := r.URL.Query().Get("window_minutes"); v != "" {
		n, err := strconv.Atoi(v)
		if err != nil || n <= 0 || time.Duration(n)*time.Minute > maxAnomalyWindow {
			httpx.WriteErr(w, apperr.E(apperr.CodeUnprocessable, apperr.Validation, "window_minutes must be 1..43200"))
			return
		}
		window = time.Duration(n) * time.Minute
	}
	rows, err := h.uc.Anomalies(r.Context(), window)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	items := make([]anomalyItem, 0, len(rows))
	for _, a := range rows {
		items = append(items, anomalyItem(a))
	}
	httpx.WriteJSON(w, http.StatusOK, items)
}

// small helpers shared by the generated-pointer models
func emailPtr(e string) *oapi_types.Email { v := oapi_types.Email(e); return &v }

func strPtr[E ~string](s string) *E { v := E(s); return &v }
