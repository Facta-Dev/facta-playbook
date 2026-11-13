# <app> — build and verification targets.
#
# pnpm is the package manager; the lockfile is never committed (the first
# install happens on the renamed copy, after scripts/rename.sh).

PNPM ?= pnpm

.DEFAULT_GOAL := help
.PHONY: help ci build test lint typecheck format format-check evals verify install

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-12s %s\n",$$1,$$2}'

install: ## Install the workspace dependencies
	$(PNPM) install

build: ## Export the Android bundle (expo export --platform android)
	$(PNPM) run export:android

test: ## Unit tests (vitest)
	$(PNPM) test

lint: ## ESLint (type-aware rules + architecture guards)
	$(PNPM) lint

typecheck: ## tsc over the app program and the test program, no emit
	$(PNPM) typecheck

format: ## Format the tree with prettier
	$(PNPM) format

format-check: ## Fail on unformatted files (prettier = gofmt analog)
	$(PNPM) format:check

evals: ## Native contract tests (node --test, zero dependencies)
	$(PNPM) evals

verify: ## Template verification script (stages a renamed copy and gates it)
	$(PNPM) verify

ci: ## The gate: typecheck + lint + test + export
	$(PNPM) run ci
