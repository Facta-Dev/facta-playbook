# <app> — build and verification targets.
# Migrations are never run at startup; `make migrate-up` (or the release
# step's one-shot migrations container) applies them. The OpenAPI contract in
# specs/ is generated from the app and committed.

UV        ?= uv
DATABASE_URL ?= postgresql+asyncpg://postgres:postgres@localhost:5432/<app>

.DEFAULT_GOAL := help
.PHONY: help tidy build fmt fmt-check vet lint test accept evals generate \
        gen-check migrate migrate-up migrate-down migrate-add run worker \
        ci verify vuln

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-16s %s\n",$$1,$$2}'

tidy: ## Create/refresh the local virtualenv (the lockfile is never committed — INV-T3)
	$(UV) sync

build: ## Build the wheel (dist/)
	$(UV) build

fmt: ## Format in place (ruff format)
	$(UV) run ruff format .

fmt-check: ## Fail on unformatted files (gofmt analog)
	$(UV) run ruff format --check .

vet: ## Typecheck without emitting (mypy strict = go vet analog)
	$(UV) run mypy src

lint: ## Ruff + import-linter (arch guards live in pyproject.toml)
	$(UV) run ruff check .
	$(UV) run lint-imports

test: ## Unit tests (no DB, no sockets)
	$(UV) run pytest tests/unit

accept: ## Behavioral contracts: the specs/features suite over the fake-wired app
	$(UV) run pytest tests/acceptance

evals: ## .claude kit contract suite (pytest, local only)
	$(UV) run pytest evals

generate: ## Regenerate specs/openapi.yaml from the app
	$(UV) run python scripts/export_openapi.py

gen-check: ## Fail if the committed OpenAPI contract is stale
	$(UV) run python scripts/export_openapi.py --check

# Alembic resolves the connection from <APP>_DATABASE_URL; the local dev
# default points at docker-compose postgres.
migrate migrate-up: ## migrate up (alias: docs say `make migrate`)
	env "<APP>_DATABASE_URL=$(DATABASE_URL)" $(UV) run alembic upgrade head

migrate-down: ## migrate down (one step)
	env "<APP>_DATABASE_URL=$(DATABASE_URL)" $(UV) run alembic downgrade -1

migrate-add: ## New migration: make migrate-add NAME=add_field
	@test -n "$(NAME)" || { echo "usage: make migrate-add NAME=<name>"; exit 1; }
	env "<APP>_DATABASE_URL=$(DATABASE_URL)" $(UV) run alembic revision -m "$(NAME)"

run: ## Run the API locally (env from shell or `. ./.env && export`)
	$(UV) run python -m app.bin.api

worker: ## Run the outbox worker (separate process from the API)
	$(UV) run python -m app.bin.worker

vuln: ## pip-audit (govulncheck analog)
	$(UV) run pip-audit

ci: ## The gate: fmt + vet + lint + gen-check + tests + evals
	$(MAKE) fmt-check
	$(MAKE) vet
	$(MAKE) lint
	$(MAKE) gen-check
	$(MAKE) test
	$(MAKE) accept
	$(MAKE) evals

verify: ## Staged renamed copy + full factory smoke
	bash scripts/verify-template.sh
