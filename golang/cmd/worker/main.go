// Command worker runs the maintenance loops as their own entry point (same
// Docker image, different entry) for scale-out deployments: the email outbox
// claims with FOR UPDATE SKIP LOCKED, so many replicas are safe, and the
// janitor's idempotent purges are equally replica-safe. In single-binary
// deployments cmd/api already runs one outbox worker in-process.
package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"sync"
	"syscall"

	"github.com/<scope>/<app>/internal/platform/config"
	"github.com/<scope>/<app>/internal/platform/email"
	"github.com/<scope>/<app>/internal/platform/pgx"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		slog.Error("fatal", "err", err)
		os.Exit(1)
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	pool, err := pgx.NewPool(ctx, cfg)
	if err != nil {
		slog.Error("fatal", "err", err)
		os.Exit(1)
	}
	defer pool.Close()

	slog.Info("email worker listening", "poll", cfg.OutboxPoll, "batch", cfg.OutboxSize)

	// Two loops on one context: the outbox drain and the janitor purge
	// (reference parity for the celery auth_session_* tasks). Neither races a
	// shutdown: the context fires first and both exit through ctx.Done().
	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		email.RunWorker(ctx, email.NewOutbox(pool), email.SelectSender(cfg, slog.Default()),
			cfg.OutboxPoll, cfg.OutboxSize)
	}()
	go func() {
		defer wg.Done()
		janitor{pool: pool}.Run(ctx, cfg.JanitorPoll)
	}()
	wg.Wait()
	slog.Info("worker stopped")
}
