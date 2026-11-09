// Command api is the composition root: config → pool → ports → slices →
// middleware → HTTP server. Everything wires here and nowhere else; slices
// never see each other.
package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/<scope>/<app>/internal/features/adminusers"
	"github.com/<scope>/<app>/internal/features/authoauth"
	"github.com/<scope>/<app>/internal/features/authpassword"
	"github.com/<scope>/<app>/internal/features/authsession"
	"github.com/<scope>/<app>/internal/features/authsignup"
	featrt "github.com/<scope>/<app>/internal/features/realtime"
	"github.com/<scope>/<app>/internal/features/widgets"
	"github.com/<scope>/<app>/internal/platform/authx"
	"github.com/<scope>/<app>/internal/platform/config"
	"github.com/<scope>/<app>/internal/platform/email"
	"github.com/<scope>/<app>/internal/platform/httpx"
	"github.com/<scope>/<app>/internal/platform/oauthx"
	"github.com/<scope>/<app>/internal/platform/pgx"
	platrt "github.com/<scope>/<app>/internal/platform/realtime"
	"github.com/<scope>/<app>/internal/shared/apperr"
)

func main() {
	if err := run(); err != nil {
		slog.Error("fatal", "err", err)
		os.Exit(1)
	}
}

func run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}
	setupLog(cfg.LogLevel)

	// -probe: container healthcheck — distroless carries no shell/curl/wget,
	// so the api binary probes its own /healthz and exits non-zero on failure.
	if len(os.Args) > 1 && os.Args[1] == "-probe" {
		if err := probeHealth(cfg.Bind); err != nil {
			slog.Error("probe", "err", err)
			os.Exit(1)
		}
		return nil
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	pool, err := pgx.NewPool(ctx, cfg)
	if err != nil {
		return err
	}
	defer pool.Close()

	verifier := authx.NewHS256Verifier(cfg.JWTSecret, "<app>")
	minter := authx.NewHS256Minter(cfg.JWTSecret, "<app>")
	hub := platrt.NewHub(0) // hub = per-subscriber buffer, hub default 16 (OutboxSize is the DB fetch size)

	cookie := ""
	if cfg.AuthCookie {
		cookie = "<app>_access_token"
	}
	mux := http.NewServeMux()

	registerHealth(mux, pool)
	authsignup.Register(mux, authsignup.Deps{
		Store: authsignup.NewPostgres(pool), Emails: email.NewOutbox(pool),
		Clock: nil, Pepper: cfg.Pepper, VerifyTTL: cfg.VerifyCodeTTL,
	})
	authsession.Register(mux, authsession.Deps{
		Store: authsession.NewPostgres(pool), Minter: minter, Verifier: verifier,
		Pepper: cfg.Pepper, CookieName: cookie, CookieSecure: cfg.Env != "dev",
		AccessTTL: cfg.AccessTokenTTL, RefreshTTL: cfg.RefreshTokenTTL,
		MaxLoginRetry: cfg.MaxLoginRetries,
	})
	authoauth.Register(mux, authoauth.Deps{
		Store:  authoauth.NewPostgres(pool),
		Google: oauthx.NewGoogleVerifier("", cfg.Offline),
		GitHub: oauthx.NewGitHubClient(cfg.Offline),
		States: oauthx.NewLinkToken(cfg.JWTSecret),
		Minter: minter, Verifier: verifier, Clock: nil,
		GoogleClientID:     cfg.GoogleClientID,
		GoogleEnabled:      cfg.GoogleEnabled,
		GitHubClientID:     cfg.GitHubClientID,
		GitHubClientSecret: cfg.GitHubSecret,
		GitHubCallbackURL:  cfg.GitHubCallback,
		GitHubEnabled:      cfg.GitHubEnabled,
		FrontendSuccess:    cfg.FrontendSuccess,
		Offline:            cfg.Offline,
		CookieName:         cookie, CookieSecure: cfg.Env != "dev",
		AccessTTL: cfg.AccessTokenTTL, RefreshTTL: cfg.RefreshTokenTTL,
	})
	authpassword.Register(mux, authpassword.Deps{
		Store: authpassword.NewPostgres(pool), Emails: email.NewOutbox(pool),
		Verifier: verifier, Pepper: cfg.Pepper, CookieName: cookie,
		ResetTTL: cfg.ResetCodeTTL, RateLimit: cfg.ResetRateLimit,
		RateWindow: cfg.ResetRateWindow, LogOutAllOnReset: cfg.LogOutAllOnReset,
	})
	adminusers.Register(mux, adminusers.Deps{
		Store: adminusers.NewPostgres(pool), Verifier: verifier, CookieName: cookie,
		Whitelist: cfg.AllowedAdminEmails,
	})
	widgets.Register(mux, widgets.Deps{
		Store: widgets.NewPostgres(pool), Publisher: hub, Verifier: verifier, CookieName: cookie,
	})
	featrt.Register(mux, featrt.Deps{Verifier: verifier, Publisher: hub})

	// fallback: any unmatched path is the JSON 404 envelope.
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		httpx.WriteErr(w, apperr.E(404, apperr.NotFound, "route not found"))
	})

	// email outbox worker runs on its own context, cancelled after shutdown.
	workerCtx, workerStop := context.WithCancel(context.Background())
	defer workerStop()
	var wg sync.WaitGroup
	wg.Add(1)
	go func() {
		defer wg.Done()
		email.RunWorker(workerCtx, email.NewOutbox(pool), email.SelectSender(cfg, slog.Default()),
			cfg.OutboxPoll, cfg.OutboxSize)
	}()

	handler := chain(mux, cfg)
	// baseCtx backs every request via BaseContext: it is cancelled AFTER the
	// graceful drain so long-lived SSE/WS handlers (whose contexts are the
	// request's) cannot pin srv.Shutdown to its full 10s timeout.
	baseCtx, baseCancel := context.WithCancel(context.Background())
	defer baseCancel()
	srv := &http.Server{
		Addr:              cfg.Bind,
		Handler:           handler,
		ReadHeaderTimeout: 10 * time.Second,
		IdleTimeout:       60 * time.Second,
		BaseContext:       func(net.Listener) context.Context { return baseCtx },
	}

	errCh := make(chan error, 1)
	go func() {
		slog.Info("listening", "addr", cfg.Bind)
		errCh <- srv.ListenAndServe()
	}()

	select {
	case <-ctx.Done():
	case err := <-errCh:
		if err != nil && !errors.Is(err, http.ErrServerClosed) {
			workerStop()
			wg.Wait()
			return err
		}
	}

	shutCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := srv.Shutdown(shutCtx); err != nil {
		slog.Error("shutdown", "err", err)
	}
	// SSE/WS handlers end only when their context does: release now that the
	// drain is over (fast requests were already served).
	baseCancel()
	workerStop()
	wg.Wait()
	slog.Info("stopped")
	return nil
}

// registerHealth mounts liveness + readiness; readyz degrades to a 503
// envelope when the database stops answering.
func registerHealth(mux *http.ServeMux, pool *pgxpool.Pool) {
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, _ *http.Request) {
		httpx.WriteJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.HandleFunc("GET /readyz", func(w http.ResponseWriter, r *http.Request) {
		if err := pgx.Readyz(r.Context(), pool, 2*time.Second); err != nil {
			httpx.WriteErr(w, apperr.E(503, apperr.InternalError, "database not ready"))
			return
		}
		httpx.WriteJSON(w, http.StatusOK, map[string]string{"status": "ready"})
	})
}

// probeHealth GETs http://127.0.0.1:<port>/healthz within 2 seconds (the
// configured bind's host part is ignored — inside the container we always
// hit loopback).
func probeHealth(bind string) error {
	_, port, err := net.SplitHostPort(bind)
	if err != nil {
		return err
	}
	client := &http.Client{Timeout: 2 * time.Second}
	resp, err := client.Get("http://127.0.0.1:" + port + "/healthz")
	if err != nil {
		return err
	}
	defer func() { _ = resp.Body.Close() }()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("healthz returned %s", resp.Status)
	}
	return nil
}

// chain wraps the mux with request-id, security headers, CORS and logging.
func chain(next http.Handler, cfg config.Config) http.Handler {
	return requestID(securityHeaders(cors(next, cfg)))
}

func setupLog(level string) {
	var lv slog.Level
	switch level {
	case "debug":
		lv = slog.LevelDebug
	case "warn":
		lv = slog.LevelWarn
	case "error":
		lv = slog.LevelError
	default:
		lv = slog.LevelInfo
	}
	slog.SetDefault(slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: lv})))
}
