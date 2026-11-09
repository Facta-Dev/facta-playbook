package acceptance

import (
	"context"
	"errors"
	"net/url"
	"sort"
	"strings"
	"time"

	"github.com/<scope>/<app>/internal/features/adminusers"
	"github.com/<scope>/<app>/internal/features/authoauth"
	"github.com/<scope>/<app>/internal/features/authpassword"
	"github.com/<scope>/<app>/internal/features/authsession"
	"github.com/<scope>/<app>/internal/features/authsignup"
	"github.com/<scope>/<app>/internal/features/widgets"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
	"github.com/<scope>/<app>/internal/shared/rolex"
	"github.com/<scope>/<app>/test/testutil"
)

// ---- shared in-memory state ----------------------------------------------

// userRecord is the canonical in-memory user from which every slice's store
// adapter projects its own row type (slices never share models, and Go
// cannot have two same-named methods with different return types, so each
// port gets its own tiny adapter over this one table).
type userRecord struct {
	id, email, firstName, lastName  string
	passwordHash                    string
	role, language, userType        string
	googleID, githubID, picture     string
	phone, address, postal          string
	isActive, isBlocked, isVerified bool
	retryCount                      int
	createdAt, lastLogin            time.Time
}

// adminAudit mirrors one audit_logs entry.
type adminAudit struct{ actor, action, entity, entityID, payload, ip, ua string }

// userTable is the single in-memory table behind the five store adapters.
type userTable struct {
	clock    *testutil.TestClock
	users    map[string]*userRecord // by id
	emailID  map[string]string      // by email
	order    []string               // insertion order of user ids
	verifs   []authsignup.Verification
	resets   []authpassword.Reset
	sessions map[string]authsession.SessionRow // by session id
	widgets  map[string]widgets.Widget
	wOrder   []string // insertion order of widget ids
	audit    []adminAudit
}

func newUserTable(clock *testutil.TestClock) *userTable {
	return &userTable{
		clock: clock, users: map[string]*userRecord{},
		emailID: map[string]string{}, sessions: map[string]authsession.SessionRow{},
		widgets: map[string]widgets.Widget{},
	}
}

// seedUser inserts a user row directly, bypassing the HTTP API (godog
// Backgrounds depend on it).
func (t *userTable) seedUser(emailAddr, password string, mod func(*userRecord)) error {
	hash, err := authx.HashPassword(password, testutil.TestPepper)
	if err != nil {
		return err
	}
	rec := userRecord{
		id: testutil.NextID(), email: emailAddr, passwordHash: hash,
		role: string(rolex.User), userType: "COMPANY", language: "en",
		isActive: true, createdAt: t.clock.Now(),
	}
	if mod != nil {
		mod(&rec)
	}
	t.users[rec.id] = &rec
	t.emailID[emailAddr] = rec.id
	t.order = append(t.order, rec.id)
	return nil
}

// sessionTouch marks one active session inactive.
func (t *userTable) sessionTouch(id string, active bool) {
	s := t.sessions[id]
	s.IsActive = active
	t.sessions[id] = s
}

// ---- per-slice store adapters --------------------------------------------

var (
	_ authsignup.Store           = signupStore{nil}
	_ authsession.Store          = sessionStore{nil}
	_ authpassword.Store         = passwordStore{nil}
	_ adminusers.Store           = adminStore{nil}
	_ widgets.Store              = widgetStore{nil}
	_ authoauth.Store            = oauthStore{nil}
	_ authoauth.GoogleVerifier   = fakeGoogle{}
	_ authoauth.GitHubClient     = fakeGitHub{}
	_ authsignup.EmailEnqueuer   = (*testutil.RecordedEmails)(nil)
	_ authpassword.EmailEnqueuer = (*testutil.RecordedEmails)(nil)
)

type signupStore struct{ t *userTable }

func signupRow(r *userRecord) authsignup.UserRow {
	return authsignup.UserRow{
		ID: r.id, Email: r.email, FirstName: r.firstName, LastName: r.lastName,
		PasswordHash: r.passwordHash, Role: r.role, Language: r.language,
		IsActive: r.isActive, IsVerified: r.isVerified,
	}
}

func (s signupStore) GetUserByEmail(_ context.Context, emailAddr string) (authsignup.UserRow, bool, error) {
	id, ok := s.t.emailID[emailAddr]
	if !ok {
		return authsignup.UserRow{}, false, nil
	}
	return signupRow(s.t.users[id]), true, nil
}

func (s signupStore) CreateUser(_ context.Context, row authsignup.UserRow) error {
	rec := &userRecord{
		id: row.ID, email: row.Email, firstName: row.FirstName, lastName: row.LastName,
		passwordHash: row.PasswordHash, role: row.Role, language: row.Language,
		userType: row.UserType, isActive: row.IsActive, isVerified: row.IsVerified,
		createdAt: s.t.clock.Now(),
	}
	s.t.users[rec.id] = rec
	s.t.emailID[rec.email] = rec.id
	s.t.order = append(s.t.order, rec.id)
	return nil
}

// CreateCompanyProfile / CreateUserProfile seed the 1:1 profile row
// (0002_profiles parity); the scenarios do not read profiles back, so the
// in-memory adapter records the insert without storing the row.
func (s signupStore) CreateCompanyProfile(_ context.Context, _ string, _ string) error {
	return nil
}

func (s signupStore) CreateUserProfile(_ context.Context, _ string, _ string) error {
	return nil
}

func (s signupStore) CreateVerification(_ context.Context, v authsignup.Verification) error {
	s.t.verifs = append(s.t.verifs, v)
	return nil
}

func (s signupStore) LatestVerification(_ context.Context, userID string) (authsignup.Verification, bool, error) {
	for i := len(s.t.verifs) - 1; i >= 0; i-- {
		if s.t.verifs[i].UserID == userID {
			return s.t.verifs[i], true, nil
		}
	}
	return authsignup.Verification{}, false, nil
}

// UseVerification consumes the code atomically: true only when the token
// matches and the row was still unused (CAS parity with the Postgres adapter).
func (s signupStore) UseVerification(_ context.Context, vID, token string) (bool, error) {
	for i := range s.t.verifs {
		if s.t.verifs[i].ID == vID {
			if s.t.verifs[i].Token != token || s.t.verifs[i].Used {
				return false, nil
			}
			s.t.verifs[i].Used = true
			return true, nil
		}
	}
	return false, nil
}

func (s signupStore) BumpVerification(_ context.Context, vID string) error {
	for i := range s.t.verifs {
		if s.t.verifs[i].ID == vID {
			s.t.verifs[i].Attempts++
		}
	}
	return nil
}

func (s signupStore) SetEmailVerified(_ context.Context, userID string) error {
	s.t.users[userID].isVerified = true
	return nil
}

type sessionStore struct{ t *userTable }

// optStr projects an unset optional column as nil (NULL parity); note the
// fake cannot distinguish "" from an absent value — the unit fakes (and the
// Postgres adapter) own that distinction.
func optStr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func optTime(t time.Time) *time.Time {
	if t.IsZero() {
		return nil
	}
	return &t
}

func sessionRow(r *userRecord) authsession.UserRow {
	return authsession.UserRow{
		ID: r.id, Email: r.email, FirstName: r.firstName, LastName: r.lastName,
		PasswordHash: r.passwordHash, Role: r.role, Language: r.language,
		UserType: r.userType, Phone: optStr(r.phone), Picture: optStr(r.picture),
		Address: optStr(r.address), Postal: optStr(r.postal),
		IsActive: r.isActive, IsBlocked: r.isBlocked, IsVerified: r.isVerified,
		RetryCount: r.retryCount, LastLogin: optTime(r.lastLogin),
		CreatedAt: r.createdAt, GoogleLinked: r.googleID != "", GithubLinked: r.githubID != "",
	}
}

func (s sessionStore) GetUserByEmail(_ context.Context, emailAddr string) (authsession.UserRow, bool, error) {
	id, ok := s.t.emailID[emailAddr]
	if !ok {
		return authsession.UserRow{}, false, nil
	}
	return sessionRow(s.t.users[id]), true, nil
}

func (s sessionStore) GetUserByID(_ context.Context, id string) (authsession.UserRow, bool, error) {
	r, ok := s.t.users[id]
	if !ok {
		return authsession.UserRow{}, false, nil
	}
	return sessionRow(r), true, nil
}

// UpdateMe mirrors the SessionUpdateMe SQL: a nil field leaves the column
// unchanged, a present pointer (even "") overwrites it, then the profile is
// re-read (update-then-GetMe parity).
func (s sessionStore) UpdateMe(_ context.Context, userID string, in authsession.UpdateMeInput) (authsession.UserRow, bool, error) {
	r, ok := s.t.users[userID]
	if !ok {
		return authsession.UserRow{}, false, nil
	}
	if in.FirstName != nil {
		r.firstName = *in.FirstName
	}
	if in.LastName != nil {
		r.lastName = *in.LastName
	}
	if in.Phone != nil {
		r.phone = *in.Phone
	}
	if in.Picture != nil {
		r.picture = *in.Picture
	}
	if in.Language != nil {
		r.language = *in.Language
	}
	if in.Address != nil {
		r.address = *in.Address
	}
	if in.Postal != nil {
		r.postal = *in.Postal
	}
	return sessionRow(r), true, nil
}

func (s sessionStore) AfterFailedLogin(_ context.Context, userID string, lockAt int) (bool, error) {
	blocked := false
	if r := s.t.users[userID]; r != nil {
		r.retryCount++
		blocked = r.retryCount >= lockAt
		if blocked {
			r.isBlocked = true
		}
	}
	return blocked, nil
}

func (s sessionStore) AfterSuccessfulLogin(_ context.Context, userID string) error {
	r := s.t.users[userID]
	r.retryCount = 0
	r.lastLogin = s.t.clock.Now()
	return nil
}

func (s sessionStore) CreateSession(_ context.Context, row authsession.SessionRow) error {
	// DB default parity: created_at/last_activity carry "now()" on insert.
	if row.CreatedAt.IsZero() {
		row.CreatedAt = s.t.clock.Now()
	}
	if row.LastActivity.IsZero() {
		row.LastActivity = s.t.clock.Now()
	}
	s.t.sessions[row.ID] = row
	return nil
}

func (s sessionStore) ActiveSessionByToken(_ context.Context, token string) (authsession.SessionUser, bool, error) {
	for _, row := range s.t.sessions {
		if row.RefreshToken != token || !row.IsActive {
			continue
		}
		// SQL parity: the rotation view only sees unexpired rows.
		if !row.ExpiresAt.After(s.t.clock.Now()) {
			continue
		}
		r := s.t.users[row.UserID]
		return authsession.SessionUser{
			SessionRow: row, Email: r.email, Role: r.role,
			UserActive: r.isActive, IsBlocked: r.isBlocked, IsVerified: r.isVerified,
		}, true, nil
	}
	return authsession.SessionUser{}, false, nil
}

// DeactivateSession claims the refresh row: true only when it was still
// active (compare-and-swap parity with the Postgres adapter).
func (s sessionStore) DeactivateSession(_ context.Context, sessionID string) (bool, error) {
	row, ok := s.t.sessions[sessionID]
	if !ok || !row.IsActive {
		return false, nil
	}
	s.t.sessionTouch(sessionID, false)
	return true, nil
}

// RotateSession mirrors the SQL: id + old token + active + unexpired rotates
// the row in place, keeping id/created_at/expires_at.
func (s sessionStore) RotateSession(_ context.Context, sessionID, oldToken string, row authsession.SessionRow) (bool, error) {
	cur, ok := s.t.sessions[sessionID]
	if !ok || cur.RefreshToken != oldToken || !cur.IsActive || !cur.ExpiresAt.After(s.t.clock.Now()) {
		return false, nil
	}
	row.ID, row.CreatedAt, row.ExpiresAt, row.UserID = cur.ID, cur.CreatedAt, cur.ExpiresAt, cur.UserID
	row.IsActive = true
	// last_activity carries now() on the SQL UPDATE; the insert default is
	// applied by CreateSession, so rotation stamps it here.
	row.LastActivity = s.t.clock.Now()
	s.t.sessions[sessionID] = row
	return true, nil
}

func (s sessionStore) DeactivateByTokenForUser(_ context.Context, userID, token string) error {
	for _, row := range s.t.sessions {
		if row.UserID == userID && row.RefreshToken == token && row.IsActive {
			s.t.sessionTouch(row.ID, false)
		}
	}
	return nil
}

// --- session-management views (SQL parity, slices/authsession) --------------

func (s sessionStore) ListSessionsByUser(_ context.Context, userID string, activeOnly bool) ([]authsession.SessionRow, error) {
	out := []authsession.SessionRow{}
	for _, row := range s.t.sessions {
		if row.UserID != userID || (activeOnly && !row.IsActive) {
			continue
		}
		out = append(out, row)
	}
	// Mirror the SQL ordering (map iteration is random).
	sort.Slice(out, func(i, j int) bool { return out[i].LastActivity.After(out[j].LastActivity) })
	return out, nil
}

func (s sessionStore) CurrentSession(_ context.Context, userID string) (authsession.SessionRow, bool, error) {
	best, found := authsession.SessionRow{}, false
	for _, row := range s.t.sessions {
		if row.UserID != userID || !row.IsActive {
			continue
		}
		if !found || row.LastActivity.After(best.LastActivity) {
			best, found = row, true
		}
	}
	return best, found, nil
}

// RevokeSessionForUser claims one active session of the user: true only when
// it was still active (compare-and-swap parity with the Postgres adapter).
func (s sessionStore) RevokeSessionForUser(_ context.Context, userID, sessionID string) (bool, error) {
	if row, ok := s.t.sessions[sessionID]; ok && row.UserID == userID && row.IsActive {
		s.t.sessionTouch(sessionID, false)
		return true, nil
	}
	return false, nil
}

func (s sessionStore) RevokeAllForUser(_ context.Context, userID string) (int, error) {
	n := 0
	for _, row := range s.t.sessions {
		if row.UserID == userID && row.IsActive {
			s.t.sessionTouch(row.ID, false)
			n++
		}
	}
	return n, nil
}

func (s sessionStore) CountExpiredActive(_ context.Context) (int, error) {
	n := 0
	for _, row := range s.t.sessions {
		if row.IsActive && !row.ExpiresAt.After(s.t.clock.Now()) {
			n++
		}
	}
	return n, nil
}

// MultiIPSessions groups active sessions since the window start and reports
// users spanning more than 3 distinct IPs (reference _MULTI_IP_THRESHOLD).
func (s sessionStore) MultiIPSessions(_ context.Context, since time.Time) ([]authsession.MultiIPRow, error) {
	saw := map[string]map[string]bool{}
	for _, row := range s.t.sessions {
		if !row.IsActive || row.LastActivity.Before(since) || row.IPAddress == "" {
			continue
		}
		if saw[row.UserID] == nil {
			saw[row.UserID] = map[string]bool{}
		}
		saw[row.UserID][row.IPAddress] = true
	}
	out := []authsession.MultiIPRow{}
	for uid, ips := range saw {
		if len(ips) > 3 {
			out = append(out, authsession.MultiIPRow{UserID: uid, IPCount: len(ips)})
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].UserID < out[j].UserID })
	return out, nil
}

type passwordStore struct{ t *userTable }

func passwordRow(r *userRecord) authpassword.UserRow {
	return authpassword.UserRow{
		ID: r.id, Email: r.email, FirstName: r.firstName, Language: r.language,
		IsActive: r.isActive, IsVerified: r.isVerified, Password: r.passwordHash,
	}
}

func (s passwordStore) GetUserByEmail(_ context.Context, emailAddr string) (authpassword.UserRow, bool, error) {
	id, ok := s.t.emailID[emailAddr]
	if !ok {
		return authpassword.UserRow{}, false, nil
	}
	return passwordRow(s.t.users[id]), true, nil
}

func (s passwordStore) GetUserByID(_ context.Context, id string) (authpassword.UserRow, bool, error) {
	r, ok := s.t.users[id]
	if !ok {
		return authpassword.UserRow{}, false, nil
	}
	return passwordRow(r), true, nil
}

func (s passwordStore) CountResetsSince(_ context.Context, emailAddr string, since time.Time) (int, error) {
	n := 0
	for _, r := range s.t.resets {
		if r.Email == emailAddr && !r.CreatedAt.Before(since) {
			n++
		}
	}
	return n, nil
}

func (s passwordStore) CreateReset(_ context.Context, r authpassword.Reset) error {
	r.CreatedAt = s.t.clock.Now()
	s.t.resets = append(s.t.resets, r)
	return nil
}

func (s passwordStore) LatestReset(_ context.Context, emailAddr string) (authpassword.Reset, bool, error) {
	for i := len(s.t.resets) - 1; i >= 0; i-- {
		if s.t.resets[i].Email == emailAddr {
			return s.t.resets[i], true, nil
		}
	}
	return authpassword.Reset{}, false, nil
}

// MarkResetUsed consumes the code atomically: true only when the token
// matches and the row was still unused (CAS parity with the Postgres adapter).
func (s passwordStore) MarkResetUsed(_ context.Context, id, token string) (bool, error) {
	for i := range s.t.resets {
		if s.t.resets[i].ID == id {
			if s.t.resets[i].Token != token || s.t.resets[i].Used {
				return false, nil
			}
			s.t.resets[i].Used = true
			return true, nil
		}
	}
	return false, nil
}

func (s passwordStore) BumpReset(_ context.Context, id string) error {
	for i := range s.t.resets {
		if s.t.resets[i].ID == id {
			s.t.resets[i].Attempts++
		}
	}
	return nil
}

func (s passwordStore) SetPasswordAndVerify(_ context.Context, id, hash string) error {
	r := s.t.users[id]
	r.passwordHash = hash
	r.isVerified = true
	return nil
}

func (s passwordStore) SetPassword(_ context.Context, id, hash string) error {
	s.t.users[id].passwordHash = hash
	return nil
}

func (s passwordStore) RevokeAllSessions(_ context.Context, userID string) error {
	for _, row := range s.t.sessions {
		if row.UserID == userID && row.IsActive {
			s.t.sessionTouch(row.ID, false)
		}
	}
	return nil
}

func (s passwordStore) RevokeOtherSessions(_ context.Context, userID, sessionID string) error {
	for _, row := range s.t.sessions {
		if row.UserID == userID && row.ID != sessionID && row.IsActive {
			s.t.sessionTouch(row.ID, false)
		}
	}
	return nil
}

type adminStore struct{ t *userTable }

func adminRow(r *userRecord) adminusers.Row {
	return adminusers.Row{
		ID: r.id, Email: r.email, FirstName: r.firstName, LastName: r.lastName,
		Role: r.role, UserType: r.userType, IsActive: r.isActive,
		IsBlocked: r.isBlocked, IsVerified: r.isVerified, Language: r.language,
		CreatedAt: r.createdAt, LastLogin: r.lastLogin,
	}
}

func (s adminStore) List(_ context.Context, limit, offset int) ([]adminusers.Row, error) {
	// ponytail: offset window over the insertion order; <100 users per scenario.
	out := []adminusers.Row{}
	for i := offset; i < len(s.t.order) && len(out) < limit; i++ {
		out = append(out, adminRow(s.t.users[s.t.order[i]]))
	}
	return out, nil
}

func (s adminStore) Count(context.Context) (int, error) { return len(s.t.order), nil }

func (s adminStore) Get(_ context.Context, id string) (adminusers.Row, bool, error) {
	r, ok := s.t.users[id]
	if !ok {
		return adminusers.Row{}, false, nil
	}
	return adminRow(r), true, nil
}

// GetByEmail mirrors the unique email column; UpgradeToAdmin moves role +
// user_type to ADMIN (bootstrap whitelist route).
func (s adminStore) GetByEmail(_ context.Context, email string) (adminusers.Row, bool, error) {
	for _, id := range s.t.order {
		r := s.t.users[id]
		if strings.EqualFold(r.email, email) {
			return adminRow(r), true, nil
		}
	}
	return adminusers.Row{}, false, nil
}

func (s adminStore) UpgradeToAdmin(_ context.Context, id string) error {
	r := s.t.users[id]
	r.role, r.userType = "ADMIN", "ADMIN"
	s.t.users[id] = r
	return nil
}

func (s adminStore) Update(_ context.Context, id string, p adminusers.Patch) error {
	r := s.t.users[id]
	if p.IsActive != nil {
		r.isActive = *p.IsActive
	}
	if p.IsBlocked != nil {
		r.isBlocked = *p.IsBlocked
	}
	if p.Role != "" {
		r.role = p.Role
	}
	return nil
}

func (s adminStore) WriteAudit(_ context.Context, actorID, action, entity, entityID, payload, ip, ua string) error {
	s.t.audit = append(s.t.audit, adminAudit{actorID, action, entity, entityID, payload, ip, ua})
	return nil
}

type widgetStore struct{ t *userTable }

func (s widgetStore) Insert(_ context.Context, w widgets.Widget) error {
	s.t.widgets[w.ID] = w
	s.t.wOrder = append(s.t.wOrder, w.ID)
	return nil
}

func (s widgetStore) List(_ context.Context, ownerID string, limit, offset int) ([]widgets.Widget, error) {
	out := []widgets.Widget{}
	for i := offset; i < len(s.t.wOrder) && len(out) < limit; i++ {
		if w := s.t.widgets[s.t.wOrder[i]]; w.OwnerUserID == ownerID {
			out = append(out, w)
		}
	}
	return out, nil
}

func (s widgetStore) Get(_ context.Context, id, ownerID string) (widgets.Widget, bool, error) {
	w, ok := s.t.widgets[id]
	if !ok || w.OwnerUserID != ownerID {
		return widgets.Widget{}, false, nil
	}
	return w, true, nil
}

func (s widgetStore) Update(_ context.Context, id, ownerID, name, kind string) error {
	w := s.t.widgets[id]
	w.Name, w.Kind = name, kind
	s.t.widgets[id] = w
	return nil
}

func (s widgetStore) Delete(_ context.Context, id, ownerID string) error {
	delete(s.t.widgets, id)
	return nil
}

// --- oauth adapters ----------------------------------------------------------

func oauthRow(r *userRecord) authoauth.User {
	return authoauth.User{
		ID: r.id, Email: r.email, FirstName: r.firstName, LastName: r.lastName,
		Picture: r.picture, Role: r.role, UserType: r.userType, Language: r.language,
		GoogleID: r.googleID, GithubID: r.githubID,
		IsActive: r.isActive, IsBlocked: r.isBlocked, IsVerified: r.isVerified,
	}
}

// oauthStore adapts the users + sessions table to the social-login port.
type oauthStore struct{ t *userTable }

func (s oauthStore) userByID(id string) (*userRecord, bool) {
	r, ok := s.t.users[id]
	return r, ok
}

func (s oauthStore) userByProvider(match func(*userRecord) bool) (*userRecord, bool) {
	for _, id := range s.t.order {
		if r := s.t.users[id]; match(r) {
			return r, true
		}
	}
	return nil, false
}

func (s oauthStore) GetUserByGoogleID(_ context.Context, googleID string) (authoauth.User, bool, error) {
	if r, ok := s.userByProvider(func(r *userRecord) bool { return r.googleID != "" && r.googleID == googleID }); ok {
		return oauthRow(r), true, nil
	}
	return authoauth.User{}, false, nil
}

func (s oauthStore) GetUserByGithubID(_ context.Context, githubID string) (authoauth.User, bool, error) {
	if r, ok := s.userByProvider(func(r *userRecord) bool { return r.githubID != "" && r.githubID == githubID }); ok {
		return oauthRow(r), true, nil
	}
	return authoauth.User{}, false, nil
}

func (s oauthStore) GetUserByEmail(_ context.Context, emailAddr string) (authoauth.User, bool, error) {
	id, ok := s.t.emailID[emailAddr]
	if !ok {
		return authoauth.User{}, false, nil
	}
	return oauthRow(s.t.users[id]), true, nil
}

func (s oauthStore) GetUserByID(_ context.Context, id string) (authoauth.User, bool, error) {
	r, ok := s.userByID(id)
	if !ok {
		return authoauth.User{}, false, nil
	}
	return oauthRow(r), true, nil
}

func (s oauthStore) CreateUser(_ context.Context, u authoauth.User) error {
	rec := &userRecord{
		id: u.ID, email: u.Email, firstName: u.FirstName, lastName: u.LastName,
		role: u.Role, language: u.Language, userType: u.UserType,
		googleID: u.GoogleID, githubID: u.GithubID, picture: u.Picture,
		isActive: u.IsActive, isVerified: u.IsVerified, createdAt: s.t.clock.Now(),
	}
	s.t.users[rec.id] = rec
	s.t.emailID[rec.email] = rec.id
	s.t.order = append(s.t.order, rec.id)
	return nil
}

// setProvider applies one provider link (verify + picture-if-empty parity
// with the SQL adapters).
func (s oauthStore) setProvider(userID, field, providerID, picture string) {
	r := s.t.users[userID]
	switch field {
	case "google":
		r.googleID = providerID
	case "github":
		r.githubID = providerID
	}
	r.isVerified = true
	if r.picture == "" {
		r.picture = picture
	}
}

func (s oauthStore) LinkGoogle(_ context.Context, userID, providerID, picture string) error {
	s.setProvider(userID, "google", providerID, picture)
	return nil
}

func (s oauthStore) LinkGithub(_ context.Context, userID, providerID, picture string) error {
	s.setProvider(userID, "github", providerID, picture)
	return nil
}

// GithubOnly attaches the github id with no side effects (link flow).
func (s oauthStore) GithubOnly(_ context.Context, userID, githubID string) error {
	s.t.users[userID].githubID = githubID
	return nil
}

// SaveGithubLogin refreshes the login-flow identity (email upgrade decided
// in Go; verified mark + picture-if-empty here).
func (s oauthStore) SaveGithubLogin(_ context.Context, userID, emailAddr, picture string) error {
	r := s.t.users[userID]
	if emailAddr != "" {
		r.email = emailAddr
		s.t.emailID[emailAddr] = userID
	}
	r.isVerified = true
	if r.picture == "" {
		r.picture = picture
	}
	return nil
}

func (s oauthStore) CreateSession(_ context.Context, row authoauth.SessionRow) error {
	s.t.sessions[row.ID] = authsession.SessionRow{
		ID: row.ID, UserID: row.UserID, RefreshToken: row.RefreshToken,
		IPAddress: row.IPAddress, UserAgent: row.UserAgent,
		ExpiresAt: row.ExpiresAt, IsActive: row.IsActive,
		CreatedAt: s.t.clock.Now(), LastActivity: s.t.clock.Now(),
	}
	return nil
}

// fakeGoogle answers a verified identity for any @ex.com credential — the
// address itself is the token (the real verifier checks RS256/JWKS).
type fakeGoogle struct{}

func (fakeGoogle) Verify(_ context.Context, credential, _ string) (oauthx.GoogleIdentity, error) {
	local, suffix, cut := strings.Cut(credential, "@")
	if !cut || suffix != "ex.com" || local == "" {
		return oauthx.GoogleIdentity{}, errors.New("invalid credential")
	}
	return oauthx.GoogleIdentity{
		Sub: "google-" + local, Email: credential,
		GivenName: local, FamilyName: "Google", Picture: "https://gravatar.test/" + local,
	}, nil
}

// fakeGitHub maps fixed codes to fixed identities: good-code provisions
// fresh@ex.com, ada-code links ada@ex.com.
type fakeGitHub struct {
	codes map[string]oauthx.GitHubUser
}

func (f fakeGitHub) Authorize(clientID, redirectURI, state string) string {
	q := url.Values{"client_id": {clientID}, "redirect_uri": {redirectURI}, "scope": {"user:email"}}
	if state != "" {
		q.Set("state", state)
	}
	return "https://github.com/login/oauth/authorize?" + q.Encode()
}

func (f fakeGitHub) Exchange(_ context.Context, code, _, _, _ string) (oauthx.GitHubUser, error) {
	if u, ok := f.codes[code]; ok {
		return u, nil
	}
	return oauthx.GitHubUser{}, errors.New("bad_verification_code")
}
