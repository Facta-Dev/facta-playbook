// github.go: the GitHub authorization-code client — authorize URL, code
// exchange, /user + /user/emails fetch, and the noreply email resolution
// that mirrors reference exactly.
package oauthx

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"
)

// Production GitHub endpoints (overridable in tests).
const (
	githubAuthorizeURL = "https://github.com/login/oauth/authorize"
	githubTokenURL     = "https://github.com/login/oauth/access_token"
	githubUserURL      = "https://api.github.com/user"
	githubEmailsURL    = "https://api.github.com/user/emails"
	githubScope        = "user:email"
)

// noreplyHost marks GitHub's placeholder addresses: reference never trusts an
// email containing/embedding it when something real is available.
const noreplyHost = "noreply.github.com"

// GitHubUser is /user plus the resolved email.
type GitHubUser struct {
	ID        string
	Login     string
	Name      string
	Email     string
	AvatarURL string
}

// GitHubClient is the thin GitHub-side HTTP client (stdlib net/http only).
type GitHubClient struct {
	Offline bool         // INV-T7: refuse before any socket
	Client  *http.Client // nil => default 15s-timeout client

	AuthorizeURL string // overridable in tests
	TokenURL     string
	UserURL      string
	EmailsURL    string
}

// NewGitHubClient builds the client with the production endpoints.
func NewGitHubClient(offline bool) *GitHubClient {
	return &GitHubClient{
		Offline:      offline,
		AuthorizeURL: githubAuthorizeURL,
		TokenURL:     githubTokenURL,
		UserURL:      githubUserURL,
		EmailsURL:    githubEmailsURL,
	}
}

// Authorize builds the consent redirect URL: client_id, redirect_uri,
// scope=user:email and (optional) state passthrough.
func (c *GitHubClient) Authorize(clientID, redirectURI, state string) string {
	q := url.Values{"client_id": {clientID}, "redirect_uri": {redirectURI}, "scope": {githubScope}}
	if state != "" {
		q.Set("state", state)
	}
	return c.AuthorizeURL + "?" + q.Encode()
}

// Exchange turns an authorization code into the GitHub identity: token
// exchange, /user, then /user/emails with the reference's exact preference order.
// A failed /user/emails is a warning, never a hard error.
func (c *GitHubClient) Exchange(ctx context.Context, code, clientID, clientSecret, redirectURI string) (GitHubUser, error) {
	if c.Offline {
		return GitHubUser{}, ErrOffline
	}
	client := c.http()
	var tok struct {
		AccessToken      string `json:"access_token"`
		Error            string `json:"error"`
		ErrorDescription string `json:"error_description"`
	}
	tokenForm := url.Values{
		"client_id":     {clientID},
		"client_secret": {clientSecret},
		"code":          {code},
		"redirect_uri":  {redirectURI},
	}
	if err := postForm(ctx, client, c.TokenURL, tokenForm, &tok); err != nil {
		return GitHubUser{}, err
	}
	if tok.AccessToken == "" {
		if tok.Error != "" {
			return GitHubUser{}, fmt.Errorf("oauthx: github %s — %s", tok.Error, tok.ErrorDescription)
		}
		return GitHubUser{}, errors.New("oauthx: github authorization failed")
	}
	var raw struct {
		ID        int64  `json:"id"`
		Login     string `json:"login"`
		Name      string `json:"name"`
		Email     string `json:"email"`
		AvatarURL string `json:"avatar_url"`
	}
	if err := getJSON(ctx, client, c.UserURL, "Bearer "+tok.AccessToken, &raw); err != nil {
		return GitHubUser{}, err
	}
	u := GitHubUser{
		ID: strconv.FormatInt(raw.ID, 10), Login: raw.Login,
		Name: raw.Name, Email: raw.Email, AvatarURL: raw.AvatarURL,
	}
	// The profile email is often the noreply placeholder — drop it.
	if strings.Contains(u.Email, noreplyHost) {
		u.Email = ""
	}
	// Always try /user/emails for the real primary email (reference parity).
	var emails []ghEmail
	if err := getJSON(ctx, client, c.EmailsURL, "Bearer "+tok.AccessToken, &emails); err != nil {
		emails = nil // scope-restricted tokens may never see it; fall through
	}
	resolveEmail(&u, emails)
	if u.Email == "" {
		u.Email = u.ID + "@users.noreply.github.com"
	}
	return u, nil
}

// ghEmail is one row of /user/emails.
type ghEmail struct {
	Email    string `json:"email"`
	Primary  bool   `json:"primary"`
	Verified bool   `json:"verified"`
}

// resolveEmail applies the reference's preference order, only when the profile left
// the email empty: primary+verified, then any verified, then the first real
// address. Noreply placeholders never qualify.
func resolveEmail(u *GitHubUser, emails []ghEmail) {
	if u.Email != "" {
		return
	}
	real := make([]ghEmail, 0, len(emails))
	for _, e := range emails {
		if strings.HasSuffix(e.Email, noreplyHost) {
			continue
		}
		real = append(real, e)
	}
	for _, e := range real {
		if e.Primary && e.Verified {
			u.Email = e.Email
			return
		}
	}
	for _, e := range real {
		if e.Verified {
			u.Email = e.Email
			return
		}
	}
	if len(real) > 0 {
		u.Email = real[0].Email
	}
}

func (c *GitHubClient) http() *http.Client {
	if c.Client != nil {
		return c.Client
	}
	return &http.Client{Timeout: 15 * time.Second}
}

// getJSON issues a GET (optional bearer) and decodes a JSON reply into dst.
func getJSON(ctx context.Context, client *http.Client, target, bearer string, dst any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, target, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", "application/json")
	if bearer != "" {
		req.Header.Set("Authorization", bearer)
	}
	return decode(client.Do, req, dst)
}

// postForm issues a form-encoded POST and decodes the JSON reply.
func postForm(ctx context.Context, client *http.Client, target string, form url.Values, dst any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, target, strings.NewReader(form.Encode()))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Set("Accept", "application/json")
	return decode(client.Do, req, dst)
}

// decode runs the request, caps the body at 1 MiB, and unmarshals 2xx JSON.
func decode(do func(*http.Request) (*http.Response, error), req *http.Request, dst any) error {
	resp, err := do(req)
	if err != nil {
		return err
	}
	defer func() { _ = resp.Body.Close() }()
	body, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return err
	}
	if resp.StatusCode/100 != 2 {
		return fmt.Errorf("oauthx: github http %d: %s", resp.StatusCode, string(body))
	}
	if dst == nil {
		return nil
	}
	return json.Unmarshal(body, dst)
}
