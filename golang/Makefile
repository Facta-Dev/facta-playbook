# <app> — build and verification targets.
# Migrations are never run at startup; `make migrate-up` (or the goose
# release step) applies them. Generated code lives in internal/gen (committed).

GO      ?= go
GOOSE_DSN ?= postgres://postgres:postgres@localhost:5432/<app>?sslmode=disable

.DEFAULT_GOAL := help
.PHONY: help tidy build fmt-check vet lint test test-int accept generate gen-check \
        migrate-up migrate-down migrate-add run ci vuln

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-16s %s\n",$$1,$$2}'

tidy: ## Refresh go.mod/go.sum after dependency edits
	$(GO) mod tidy

build: ## Compile everything
	$(GO) build ./...

fmt-check: ## Fail on non-gofmt files
	@test -z "$$(gofmt -l .)" || { echo "gofmt needed on:"; gofmt -l .; exit 1; }

vet: ## go vet
	$(GO) vet ./...

lint: ## golangci-lint (arch guards live in .golangci.yml)
	golangci-lint run

test: ## Unit tests + hermetic acceptance (no DB needed)
	$(GO) test ./internal/... ./cmd/... ./test/...

test-int: ## Adapter + migration tests against testcontainers Postgres
	$(GO) test -tags=integration ./internal/... ./test/...

accept: ## godog acceptance suite only
	$(GO) test ./test/acceptance/... -run TestAcceptance

generate: ## sqlc (internal/gen/db) + oapi-codegen models (internal/gen/api)
	$(GO) tool sqlc generate
	$(GO) tool oapi-codegen -config specs/api.gen.yaml specs/openapi.yaml

gen-check: ## Fail if committed generated code is stale
	@rm -rf /tmp/<app>-gencheck
	@cp -R internal/gen /tmp/<app>-gencheck
	@$(MAKE) --no-print-directory generate
	@diff -ru /tmp/<app>-gencheck internal/gen || { \
	  echo "generated code stale — run make generate and commit"; exit 1; }

# goose v3.29 resolves the driver/connection/dir from the env keys only —
# the legacy `-dir migrations "dsn" up` flag form misparses.
migrate migrate-up: ## goose up (alias: docs say `make migrate`)
	GOOSE_DRIVER=postgres GOOSE_DBSTRING="$(GOOSE_DSN)" GOOSE_MIGRATION_DIR=migrations $(GO) tool goose up

migrate-down: ## goose down (one step)
	GOOSE_DRIVER=postgres GOOSE_DBSTRING="$(GOOSE_DSN)" GOOSE_MIGRATION_DIR=migrations $(GO) tool goose down

migrate-add: ## New migration: make migrate-add NAME=add_field
	@test -n "$(NAME)" || { echo "usage: make migrate-add NAME=<name>"; exit 1; }
	@ts=$$(date +%Y%m%d%H%M%S); \
	  printf -- '-- +goose Up\n-- %s\n\n-- +goose Down\n\n' '$(NAME)' > migrations/$$ts_$(NAME).sql; \
	  echo "created migrations/$$ts_$(NAME).sql"

run: ## Run the API locally (env from shell or `. ./.env && export` not needed)
	$(GO) run ./cmd/api

vuln: ## govulncheck the module (stdlib + deps)
	$(GO) tool govulncheck ./...

ci: ## The gate: fmt + vet + lint + gen-check + tests
	$(MAKE) fmt-check
	$(MAKE) vet
	$(MAKE) lint
	$(MAKE) gen-check
	$(MAKE) test