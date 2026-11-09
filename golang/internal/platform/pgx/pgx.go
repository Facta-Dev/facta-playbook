// Package pgx owns the Postgres connection: pool bootstrap and the readyz
// probe. Adapters in the feature slices share this pool.
package pgx

import (
	"context"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/<scope>/<app>/internal/platform/config"
)

// NewPool opens a pgx pool from the database URL and verifies it with Ping.
func NewPool(ctx context.Context, cfg config.Config) (*pgxpool.Pool, error) {
	pc, err := pgxpool.ParseConfig(cfg.DatabaseURL)
	if err != nil {
		return nil, err
	}
	pc.MaxConns = cfg.DBMaxConns
	pool, err := pgxpool.NewWithConfig(ctx, pc)
	if err != nil {
		return nil, err
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, err
	}
	return pool, nil
}

// Readyz pings the database inside the timeout; the caller answers 503 when
// it fails (SELECT-1 semantics via the pool's connection check).
func Readyz(ctx context.Context, pool *pgxpool.Pool, timeout time.Duration) error {
	pctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	return pool.Ping(pctx)
}
