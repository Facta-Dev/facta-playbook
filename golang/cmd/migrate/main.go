// Command migrate applies the pending goose migrations against
// <APP>_DATABASE_URL and exits. It is the compose migrations service (dev)
// and the release-step runner (prod); the API never migrates itself —
// replicas would race a schema change (see the Makefile doctrine note).
// On the host, `make migrate-up` runs the goose CLI directly instead.
package main

import (
	"context"
	"database/sql"
	"fmt"
	"os"
	"time"

	// Registered driver ("pgx") for database/sql; used only for its side effect.
	_ "github.com/jackc/pgx/v5/stdlib"
	"github.com/pressly/goose/v3"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "migrate:", err)
		os.Exit(1)
	}
}

func run() error {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()
	dsn := os.Getenv("<APP>_DATABASE_URL")
	if dsn == "" {
		return fmt.Errorf("<APP>_DATABASE_URL is required")
	}
	db, err := sql.Open("pgx", dsn)
	if err != nil {
		return err
	}
	defer func() { _ = db.Close() }()
	provider, err := goose.NewProvider(goose.DialectPostgres, db, os.DirFS(migrationsDir()))
	if err != nil {
		return err
	}
	applied, err := provider.Up(ctx)
	if err != nil {
		return err
	}
	if len(applied) == 0 {
		fmt.Println("migrate: already at head")
		return nil
	}
	fmt.Printf("migrate: applied %d migration(s)\n", len(applied))
	return nil
}

// migrationsDir points at the read-only ./migrations the compose services
// bind-mount into the container; <APP>_MIGRATIONS_DIR overrides.
func migrationsDir() string {
	if d := os.Getenv("<APP>_MIGRATIONS_DIR"); d != "" {
		return d
	}
	return "/migrations"
}
