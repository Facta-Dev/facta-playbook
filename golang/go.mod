// <app> backend — Go module with literal <scope> placeholders.
//
// INV-T3: no go.sum is committed. The placeholder module path cannot resolve
// until `bash scripts/rename.sh <new-app-slug> <new-org-scope>` runs, so the
// first real build step is `go mod tidy` (reads these pins as the minimum,
// resolves transitive deps, and writes go.sum). Commit go.mod + go.sum after
// the rename. Bump pins with `go get -u ./...`.
module github.com/<scope>/<app>

go 1.27

require (
	github.com/caarlos0/env/v11 v11.4.1
	github.com/coder/websocket v1.8.15
	github.com/cucumber/godog v0.16.0
	github.com/go-playground/validator/v10 v10.30.5
	github.com/golang-jwt/jwt/v5 v5.3.1
	github.com/google/go-cmp v0.7.0
	github.com/google/uuid v1.6.0
	github.com/oapi-codegen/oapi-codegen/v2 v2.8.0 // tool pin — matches internal/gen/api
	github.com/oapi-codegen/runtime v1.7.0
	github.com/jackc/pgx/v5 v5.11.0
	github.com/pressly/goose/v3 v3.29.0
	github.com/stretchr/testify v1.12.1
	github.com/testcontainers/testcontainers-go v0.44.0
	github.com/testcontainers/testcontainers-go/modules/postgres v0.44.0
	golang.org/x/crypto v0.58.0
	golang.org/x/vuln v1.1.4
)

tool (
	github.com/oapi-codegen/oapi-codegen/v2/cmd/oapi-codegen
	github.com/pressly/goose/v3/cmd/goose
	github.com/sqlc-dev/sqlc/cmd/sqlc
	golang.org/x/vuln/cmd/govulncheck
)