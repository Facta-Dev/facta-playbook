package main

import (
	"context"
	"log/slog"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

// janitor is the worker-side housekeeping loop — reference parity for the celery
// auth_session_* tasks (expired-session cleanup + stale-code purge; the
// anomaly aggregate lives at the admin endpoint, computed on demand).
type janitor struct {
	pool *pgxpool.Pool
}

// purges lists the delete statements run on every tick: only expired rows go
// (sessions past expiry still marked active; codes past their lifetime).
// Rotation/revocation history is preserved until it expires. Deliberately
// simple — switch the code tables to a used-at purge if they accumulate.
var purges = []struct{ label, sql string }{
	{"sessions", `DELETE FROM sessions WHERE is_active AND expires_at < now()`},
	{"email_verifications", `DELETE FROM email_verifications WHERE expires_at < now()`},
	{"password_resets", `DELETE FROM password_resets WHERE expires_at < now()`},
}

// Run purges every tick until the context is cancelled; shutdown is the
// ticker's ctx.Done, same discipline as the outbox loop.
func (j janitor) Run(ctx context.Context, tick time.Duration) {
	if tick <= 0 {
		tick = time.Hour
	}
	t := time.NewTicker(tick)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-t.C:
			j.purge(ctx)
		}
	}
}

// purge runs the delete set; a failing table skips only itself so one bad
// statement never starves the others.
func (j janitor) purge(ctx context.Context) {
	for _, p := range purges {
		ct, err := j.pool.Exec(ctx, p.sql)
		if err != nil {
			slog.Error("janitor", "table", p.label, "err", err)
			continue
		}
		if n := ct.RowsAffected(); n > 0 {
			slog.Info("janitor", "table", p.label, "purged", n)
		}
	}
}
