package authoauth

import (
	"context"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgtype"
	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/<scope>/<app>/internal/gen/db"
)

// Postgres adapts the sqlc-generated queries to this slice's Store port.
type Postgres struct {
	q *db.Queries
}

// NewPostgres builds the adapter over the shared pool.
func NewPostgres(pool *pgxpool.Pool) *Postgres { return &Postgres{q: db.New(pool)} }

var _ Store = (*Postgres)(nil)

func (p *Postgres) GetUserByGoogleID(ctx context.Context, googleID string) (User, bool, error) {
	row, err := p.q.AuthoauthGetUserByGoogleID(ctx, text(googleID))
	return userFrom(row.ID, row.Email, row.FirstName, row.LastName, row.Role, row.UserType,
		row.Language, row.ProfilePicture, row.GoogleID, row.GithubID,
		row.IsActive, row.IsBlocked, row.IsVerified, err)
}

func (p *Postgres) GetUserByGithubID(ctx context.Context, githubID string) (User, bool, error) {
	row, err := p.q.AuthoauthGetUserByGithubID(ctx, text(githubID))
	return userFrom(row.ID, row.Email, row.FirstName, row.LastName, row.Role, row.UserType,
		row.Language, row.ProfilePicture, row.GoogleID, row.GithubID,
		row.IsActive, row.IsBlocked, row.IsVerified, err)
}

func (p *Postgres) GetUserByEmail(ctx context.Context, email string) (User, bool, error) {
	row, err := p.q.AuthoauthGetUserByEmail(ctx, email)
	return userFrom(row.ID, row.Email, row.FirstName, row.LastName, row.Role, row.UserType,
		row.Language, row.ProfilePicture, row.GoogleID, row.GithubID,
		row.IsActive, row.IsBlocked, row.IsVerified, err)
}

func (p *Postgres) GetUserByID(ctx context.Context, id string) (User, bool, error) {
	row, err := p.q.AuthoauthGetUserByID(ctx, uid(id))
	return userFrom(row.ID, row.Email, row.FirstName, row.LastName, row.Role, row.UserType,
		row.Language, row.ProfilePicture, row.GoogleID, row.GithubID,
		row.IsActive, row.IsBlocked, row.IsVerified, err)
}

func (p *Postgres) CreateUser(ctx context.Context, u User) error {
	return p.q.AuthoauthCreateUser(ctx, db.AuthoauthCreateUserParams{
		ID: uid(u.ID), Email: u.Email,
		FirstName: u.FirstName, LastName: u.LastName,
		Password: u.Password, UserType: u.UserType,
		ProfilePicture: text(u.Picture), GoogleID: text(u.GoogleID), GithubID: text(u.GithubID),
	})
}

func (p *Postgres) LinkGoogle(ctx context.Context, userID, providerID, picture string) error {
	return p.q.AuthoauthLinkGoogle(ctx, db.AuthoauthLinkGoogleParams{
		ID: uid(userID), GoogleID: text(providerID), Picture: text(picture),
	})
}

func (p *Postgres) LinkGithub(ctx context.Context, userID, providerID, picture string) error {
	return p.q.AuthoauthLinkGithub(ctx, db.AuthoauthLinkGithubParams{
		ID: uid(userID), GithubID: text(providerID), Picture: text(picture),
	})
}

func (p *Postgres) GithubOnly(ctx context.Context, userID, githubID string) error {
	return p.q.AuthoauthGithubOnly(ctx, db.AuthoauthGithubOnlyParams{
		ID: uid(userID), GithubID: text(githubID),
	})
}

func (p *Postgres) SaveGithubLogin(ctx context.Context, userID, email, picture string) error {
	return p.q.AuthoauthSaveGithubLogin(ctx, db.AuthoauthSaveGithubLoginParams{
		ID: uid(userID), Email: email, Picture: text(picture),
	})
}

func (p *Postgres) CreateSession(ctx context.Context, s SessionRow) error {
	return p.q.AuthoauthCreateSession(ctx, db.AuthoauthCreateSessionParams{
		ID:           uid(s.ID),
		UserID:       uid(s.UserID),
		RefreshToken: s.RefreshToken,
		IpAddress:    text(s.IPAddress),
		UserAgent:    text(s.UserAgent),
		ExpiresAt:    ts(s.ExpiresAt),
	})
}

// userFrom maps one generated row onto the slice projection; pgx.ErrNoRows
// reads as a miss. All four getters share the same column list.
func userFrom(id pgtype.UUID, email, firstName, lastName, role, userType, language string,
	picture, googleID, githubID pgtype.Text, isActive, isBlocked, isVerified bool,
	err error) (User, bool, error) {
	if err == pgx.ErrNoRows {
		return User{}, false, nil
	}
	if err != nil {
		return User{}, false, err
	}
	return User{
		ID: fromPG(id), Email: email, FirstName: firstName, LastName: lastName,
		Password: "", // never read back by this slice
		Picture:  textStr(picture),
		Role:     role, UserType: userType, Language: language,
		GoogleID: textStr(googleID), GithubID: textStr(githubID),
		IsActive: isActive, IsBlocked: isBlocked, IsVerified: isVerified,
	}, true, nil
}

// fromPG converts a pgtype.UUID back to its string form.
func fromPG(u pgtype.UUID) string {
	if !u.Valid {
		return ""
	}
	return uuid.UUID(u.Bytes).String()
}

// uid parses a string id into pgtype.UUID; invalid input yields NULL.
func uid(s string) pgtype.UUID {
	u, err := uuid.Parse(s)
	if err != nil {
		return pgtype.UUID{}
	}
	return pgtype.UUID{Bytes: u, Valid: true}
}

// text wraps a nullable string column; "" becomes NULL.
func text(s string) pgtype.Text { return pgtype.Text{String: s, Valid: s != ""} }

// textStr unwraps a nullable string column.
func textStr(t pgtype.Text) string { return t.String }

// ts wraps a time for pgtype.Timestamptz.
func ts(t time.Time) pgtype.Timestamptz { return pgtype.Timestamptz{Time: t, Valid: true} }
