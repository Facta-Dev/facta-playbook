//go:build integration

// Integration proves the Postgres side of the template: the pool, the
// goose-migration set, real postgres.go adapters from different slices, the
// email outbox claim loop and the realtime hub, against a throwaway
// testcontainers database. Requires Docker; run with:
//
//	go test -tags=integration ./test/integration
package integration

import (
	"context"
	"encoding/json"
	"log/slog"
	"os"
	"testing"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/stdlib"
	"github.com/pressly/goose/v3"
	"github.com/stretchr/testify/require"
	tcemodules "github.com/testcontainers/testcontainers-go/modules/postgres"

	"github.com/<scope>/<app>/internal/features/authsession"
	"github.com/<scope>/<app>/internal/features/authsignup"
	"github.com/<scope>/<app>/internal/features/widgets"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/config"
	"github.com/<scope>/<app>/internal/platform/email"
	pgxpkg "github.com/<scope>/<app>/internal/platform/pgx"
	platrt "github.com/<scope>/<app>/internal/platform/realtime"
)

// testConfig builds the literal config the pool needs (no env lookup).
func testConfig(dsn string) config.Config {
	return config.Config{
		DatabaseURL: dsn,
		DBMaxConns:  4,
	}
}

// TestPostgresRoundTrip boots a Postgres container, applies the migration
// set with the goose library API, then round-trips the real adapters.
func TestPostgresRoundTrip(t *testing.T) {
	ctx := context.Background()
	pgc, err := tcemodules.Run(ctx, "postgres:17-alpine",
		tcemodules.WithDatabase("demo"), tcemodules.WithUsername("demo"),
		tcemodules.WithPassword("demo"), tcemodules.BasicWaitStrategies())
	require.NoError(t, err)
	t.Cleanup(func() { _ = pgc.Terminate(ctx) })

	dsn, err := pgc.ConnectionString(ctx, "sslmode=disable")
	require.NoError(t, err)
	pool, err := pgxpkg.NewPool(ctx, testConfig(dsn))
	require.NoError(t, err)
	defer pool.Close()

	// Goose via the library API (no CLI): pgx needs the stdlib database/sql
	// driver wrapper for the provider.
	sqlDB := stdlib.OpenDB(*pool.Config().ConnConfig)
	t.Cleanup(func() { _ = sqlDB.Close() })
	prov, err := goose.NewProvider(goose.DialectPostgres, sqlDB, os.DirFS("../../migrations"))
	require.NoError(t, err)
	_, err = prov.Up(ctx)
	require.NoError(t, err)

	// authsignup adapter: create + read-back by email.
	signups := authsignup.NewPostgres(pool)
	hash, err := authx.HashPassword("s3cure-pass-123", "")
	require.NoError(t, err)
	uid := uuid.NewString()
	row := authsignup.UserRow{ID: uid, Email: "int@ex.com", PasswordHash: hash,
		UserType: "COMPANY", Role: "USER", Language: "en", IsActive: true}
	require.NoError(t, signups.CreateUser(ctx, row))
	got, found, err := signups.GetUserByEmail(ctx, "int@ex.com")
	require.NoError(t, err)
	require.True(t, found, "user readable back by email after migrations")
	require.Equal(t, uid, got.ID)

	// authsession over real pgx: create → lookup → in-place rotation under
	// the compare-and-swap. The first version of the port omitted
	// refresh_token from the lookup SELECT, so Refresh passed "" as the old
	// token and the CAS always lost — fakes masked it; integration is the
	// tier that catches it.
	ss := authsession.NewPostgres(pool)
	sRow := authsession.SessionRow{ID: uuid.NewString(), UserID: uid, RefreshToken: "rt-1",
		ExpiresAt: time.Now().UTC().Add(time.Hour), IPAddress: "10.0.0.1", IsActive: true}
	require.NoError(t, ss.CreateSession(ctx, sRow))
	s, found, err := ss.ActiveSessionByToken(ctx, "rt-1")
	require.NoError(t, err)
	require.True(t, found, "login-shape row is resolvable by its refresh token")
	require.Equal(t, "rt-1", s.RefreshToken,
		"the lookup must surface refresh_token — without it the rotation CAS always loses")
	rotated := sRow
	rotated.RefreshToken = "rt-2"
	claimed, err := ss.RotateSession(ctx, sRow.ID, "rt-1", rotated)
	require.NoError(t, err)
	require.True(t, claimed)
	claimed, err = ss.RotateSession(ctx, sRow.ID, "rt-1", rotated)
	require.NoError(t, err)
	require.False(t, claimed, "replayed old token cannot rotate twice")
	s, found, err = ss.ActiveSessionByToken(ctx, "rt-1")
	require.NoError(t, err)
	require.False(t, found, "replayed old token is dead after in-place rotation")
	s, found, err = ss.ActiveSessionByToken(ctx, "rt-2")
	require.NoError(t, err)
	require.True(t, found, "the new token is the live one")

	// authsession.UpdateMe over real pgx: nil-means-keep partial update plus
	// the update-then-GetMe re-read (reference PUT /account/me parity). The
	// fakes mirror the COALESCE SQL by hand; only this tier proves the narg
	// params + the adapter mapping against real column types.
	first, phoneIn := "Ada", "+54 11 5555-5100"
	profile, updFound, err := ss.UpdateMe(ctx, uid, authsession.UpdateMeInput{FirstName: &first, Phone: &phoneIn})
	require.NoError(t, err)
	require.True(t, updFound)
	require.Equal(t, "Ada", profile.FirstName)
	require.NotNil(t, profile.Phone)
	require.Equal(t, phoneIn, *profile.Phone)
	require.Equal(t, "en", profile.Language, "a nil field keeps the row's current language")
	require.Equal(t, "", profile.LastName, "the '' default is kept, not coerced to NULL")
	require.Equal(t, "COMPANY", profile.UserType, "the re-read must surface the widened row")
	require.False(t, profile.CreatedAt.IsZero(), "created_at rides the re-read")

	// Clearing: a present empty pointer sets '' (never NULL) — presence is
	// expressed by the pointer, and textSet marks any non-nil pointer valid.
	empty := ""
	profile, updFound, err = ss.UpdateMe(ctx, uid, authsession.UpdateMeInput{Phone: &empty})
	require.NoError(t, err)
	require.True(t, updFound)
	require.NotNil(t, profile.Phone)
	require.Equal(t, "", *profile.Phone, "a present empty pointer sets '' (never NULL)")

	// unknown id: 0 affected rows reads as not-found.
	_, updFound, err = ss.UpdateMe(ctx, uuid.NewString(), authsession.UpdateMeInput{})
	require.NoError(t, err)
	require.False(t, updFound, "an unknown user id reads as not-found")

	// 0002 Profiles: both 1:1 profile inserts resolve against the real
	// tables (the CHECK on users.user_type, the profile FKs, the defaults).
	require.NoError(t, signups.CreateCompanyProfile(ctx, uid, "Int Company"))
	require.NoError(t, signups.CreateUserProfile(ctx, uid, "Int User"))

	// widgets adapter: insert + owner-scoped list (the FK needs the user).
	wstore := widgets.NewPostgres(pool)
	w := widgets.Widget{ID: uuid.NewString(), Name: "int-widget", Kind: "sprocket",
		OwnerUserID: uid, CreatedAt: time.Now().UTC()}
	require.NoError(t, wstore.Insert(ctx, w))
	ws, err := wstore.List(ctx, uid, 20, 0)
	require.NoError(t, err)
	require.Len(t, ws, 1)
	require.Equal(t, "int-widget", ws[0].Name)
	ws, err = wstore.List(ctx, uuid.NewString(), 20, 0)
	require.NoError(t, err)
	require.Empty(t, ws, "another owner must not see the widget")

	// email outbox: enqueue -> claim -> console-send -> complete -> empty.
	outbox := email.NewOutbox(pool)
	require.NoError(t, outbox.Enqueue(ctx, email.Message{
		To: "int@ex.com", TemplateKey: "verify_email",
		Vars: map[string]any{"code": "123456", "locale": "en"},
	}))
	jobs, err := outbox.Claim(ctx, time.Now(), 10)
	require.NoError(t, err)
	require.Len(t, jobs, 1)
	subject, body, err := email.Render(jobs[0].TemplateKey, jobs[0].Vars, "en")
	require.NoError(t, err)
	require.NotEmpty(t, subject)
	sender := email.NewConsoleSender(slog.Default())
	require.NoError(t, sender.Send(ctx, jobs[0].To, subject, body))
	require.NoError(t, outbox.Complete(ctx, jobs[0].ID))
	jobs, err = outbox.Claim(ctx, time.Now(), 10)
	require.NoError(t, err)
	require.Empty(t, jobs, "the completed job must not be claimed again")

	// realtime: one Publish lands on the subscribed room channel.
	hub := platrt.NewHub(4)
	ch, cancel := hub.Subscribe(platrt.RoomUser(uid))
	defer cancel()
	hub.Publish(platrt.Event{Room: platrt.RoomUser(uid), Type: "widget.created",
		Payload: map[string]any{"id": w.ID}})
	select {
	case ev := <-ch:
		raw, _ := json.Marshal(ev.Payload)
		require.Contains(t, string(raw), w.ID)
	case <-time.After(2 * time.Second):
		t.Fatal("realtime Publish did not reach the Subscribe channel")
	}
}
