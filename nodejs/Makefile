# <app> — build and verification targets.
# Migrations are never run at startup; `make migrate-up` (or the release
# step's one-shot migrations container) applies them. Generated code lives
# in src/gen (committed).

PNPM      ?= pnpm
DATABASE_URL ?= postgres://postgres:postgres@localhost:5432/<app>?sslmode=disable

.DEFAULT_GOAL := help
.PHONY: help tidy build fmt-check vet lint test generate gen-check \
        migrate migrate-up migrate-down migrate-add run ci vuln

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-16s %s\n",$$1,$$2}'

tidy: ## Refresh node_modules after dependency edits (the lockfile is never committed — INV-T3)
	$(PNPM) install

build: ## Compile TypeScript to dist/
	$(PNPM) build

fmt-check: ## Fail on unformatted files (prettier = gofmt analog)
	$(PNPM) format:check

vet: ## Typecheck without emitting (tsc --noEmit = go vet analog)
	$(PNPM) typecheck

lint: ## ESLint (arch guards live in eslint.config.mjs)
	$(PNPM) lint

test: ## Unit tests (no DB needed)
	$(PNPM) test

generate: ## openapi-typescript models (src/gen/api/schema.d.ts)
	$(PNPM) gen

gen-check: ## Fail if committed generated code is stale
	@rm -rf /tmp/<app>-gencheck
	@cp -R src/gen/api /tmp/<app>-gencheck
	@$(MAKE) --no-print-directory generate
	@diff -ru /tmp/<app>-gencheck src/gen/api || { \
	  echo "generated code stale — run make generate and commit"; exit 1; }

# node-pg-migrate resolves the connection from DATABASE_URL; the local dev
# default points at docker-compose postgres.
migrate migrate-up: ## migrate up (alias: docs say `make migrate`)
	DATABASE_URL="$(DATABASE_URL)" $(PNPM) exec node-pg-migrate -m migrations up

migrate-down: ## migrate down (one step)
	DATABASE_URL="$(DATABASE_URL)" $(PNPM) exec node-pg-migrate -m migrations down

migrate-add: ## New migration: make migrate-add NAME=add_field
	@test -n "$(NAME)" || { echo "usage: make migrate-add NAME=<name>"; exit 1; }
	@ts=$$(date +%Y%m%d%H%M%S); \
	  printf "// %s — fill in.\n\n/** @param {import('node-pg-migrate').MigrationBuilder} pgm */\nexport const up = (pgm) => {};\n\n/** @param {import('node-pg-migrate').MigrationBuilder} pgm */\nexport const down = (pgm) => {};\n" '$(NAME)' > migrations/$$ts_$(NAME).mjs; \
	  echo "created migrations/$$ts_$(NAME).mjs"

run: ## Run the API locally (env from shell or `. ./.env && export`)
	$(PNPM) build && node dist/bin/api.js

vuln: ## pnpm audit (govulncheck analog)
	$(PNPM) audit --audit-level=high

ci: ## The gate: fmt + vet + lint + gen-check + tests
	$(MAKE) fmt-check
	$(MAKE) vet
	$(MAKE) lint
	$(MAKE) gen-check
	$(MAKE) test

accept: ## Behavioral contracts: the specs/features suite over the fake-wired app (godog parity)
	$(PNPM) accept
