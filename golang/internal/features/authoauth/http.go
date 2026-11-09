package authoauth

import (
	"errors"
	"net/http"
	"net/url"
	"strings"
	"time"

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

// google: POST /api/v1/auth/google — One-Tap credential in, login shape out.
func (h handlers) google(w http.ResponseWriter, r *http.Request) {
	var req api.GoogleAuthRequest
	if httpx.Decode(w, r, &req) != nil {
		httpx.WriteErr(w, ErrValidation)
		return
	}
	res, err := h.uc.Google(r.Context(), req.Credential, httpx.RequestIP(r), r.UserAgent())
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	h.setCookie(w, res.AccessToken)
	httpx.WriteJSON(w, http.StatusOK, h.tokenPair(res))
}

// githubLogin: GET /api/v1/auth/github/login — 302 to GitHub's authorize
// URL when enabled; a disabled/unconfigured provider answers the 403
// envelope (operator problem, not a user-facing redirect).
func (h handlers) githubLogin(w http.ResponseWriter, r *http.Request) {
	if err := h.uc.githubGate(); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	target, err := h.uc.GithubLoginURL(r.URL.Query().Get("state"))
	if err != nil {
		h.redirectError(w, r, errGithubNotConfig, errText(err))
		return
	}
	http.Redirect(w, r, target, http.StatusFound)
}

// githubCallback: GET /api/v1/auth/github/callback — link flow or login
// flow; EVERY outcome is a 302 (tokens in the fragment on success). A
// disabled/unconfigured provider answers the 403 envelope instead.
func (h handlers) githubCallback(w http.ResponseWriter, r *http.Request) {
	if err := h.uc.githubGate(); err != nil {
		httpx.WriteErr(w, err)
		return
	}
	q := r.URL.Query()
	errorCode, description := q.Get("error"), q.Get("error_description")
	// GitHub sends its own consent errors straight back to us.
	if errorCode == "" && q.Get("code") == "" {
		errorCode, description = errNoCode, ""
	}
	if errorCode != "" {
		h.redirectError(w, r, errorCode, description)
		return
	}
	state := q.Get("state")
	if strings.HasPrefix(state, LinkStatePrefix) {
		if err := h.uc.GithubLink(r.Context(), strings.TrimPrefix(state, LinkStatePrefix), q.Get("code")); err != nil {
			h.redirectError(w, r, errAuthFailed, errText(err))
			return
		}
		http.Redirect(w, r, h.uc.linkSuccessURL(), http.StatusFound)
		return
	}
	res, err := h.uc.GithubLogin(r.Context(), q.Get("code"), httpx.RequestIP(r), r.UserAgent())
	if err != nil {
		h.redirectError(w, r, errAuthFailed, errText(err))
		return
	}
	fragment := url.Values{
		"access_token":  {res.AccessToken},
		"refresh_token": {res.RefreshToken},
	}
	http.Redirect(w, r, h.uc.frontendBase()+"#"+fragment.Encode(), http.StatusFound)
}

// redirectError bounces to the frontend success URL with ?error= pairs.
func (h handlers) redirectError(w http.ResponseWriter, r *http.Request, code, description string) {
	q := url.Values{"error": {code}, "error_description": {description}}
	http.Redirect(w, r, h.uc.frontendBase()+"?"+q.Encode(), http.StatusFound)
}

// githubConnect: GET /api/v1/auth/github/connect-url — bearer-protected
// {url} whose state carries the link JWT (reference /account/github/connect-url).
func (h handlers) githubConnect(w http.ResponseWriter, r *http.Request) {
	id, _ := authx.IdentityFrom(r.Context())
	target, err := h.uc.GithubConnectURL(id.UserID)
	if err != nil {
		httpx.WriteErr(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, api.GithubConnectURLResponse{Url: target})
}

// errText prefers the human detail over the code-prefixed Error() for the
// redirect's error_description.
func errText(err error) string {
	var ae *apperr.AppError
	if errors.As(err, &ae) {
		return ae.Detail
	}
	return err.Error()
}
