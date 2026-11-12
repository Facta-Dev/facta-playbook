# <app> — build and verification targets.
#
# pnpm is the package manager; the lockfile is never committed (the first
# install happens on the renamed copy, after scripts/rename.sh).

PNPM ?= pnpm

.DEFAULT_GOAL := help
.PHONY: help ci build test lint format format-check verify

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-12s %s\n",$$1,$$2}'

build: ## Bundle the unpacked extension into dist/
	$(PNPM) build

test: ## Unit tests (vitest)
	$(PNPM) test

lint: ## ESLint + dependency-cruiser architecture rules
	$(PNPM) lint

format: ## Format the tree with prettier
	$(PNPM) format

format-check: ## Fail on unformatted files (prettier = gofmt analog)
	$(PNPM) format:check

verify: ## Template verification script
	$(PNPM) verify

ci: ## The gate: typecheck + lint + test + build
	$(PNPM) run ci
