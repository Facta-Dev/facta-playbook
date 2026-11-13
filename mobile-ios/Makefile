# <app> — build and verification targets.
#
# The portable gates run on any OS; the Apple-toolchain tier needs macOS +
# Xcode and prints a visible SKIP elsewhere.

.DEFAULT_GOAL := help
.PHONY: help generate build test evals lint clean verify ci

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-10s %s\n",$$1,$$2}'

generate: ## Generate the Xcode project from project.yml (macOS + XcodeGen)
	xcodegen generate

build: ## Build the app for the simulator (macOS + Xcode)
	bash scripts/native-gates.sh build

test: ## Run every Swift package test (macOS + Xcode toolchain)
	bash scripts/native-gates.sh test

evals: ## Run the .claude contract tests (node --test, zero dependencies)
	node --test evals/contracts.test.mjs

lint: ## Static gates: architecture, plus the .claude linters once renamed
	bash scripts/check-architecture.sh
	@if grep -qE '<(app|APP|scope)>' AGENTS.md; then \
	  echo "lint: template placeholders present — the .claude linters run on the renamed copy in 'make verify'"; \
	else \
	  bash scripts/lint-agents.sh && \
	  bash scripts/lint-skills.sh && \
	  bash scripts/lint-commands.sh && \
	  bash scripts/instruction-budget.sh; \
	fi

clean: ## Remove generated Xcode and SwiftPM output
	rm -rf build DerivedData *.xcodeproj
	find Packages -name .build -type d -prune -exec rm -rf {} +

verify: ## Stage a renamed copy and run every gate on it
	bash scripts/verify-template.sh

ci: ## The gate: lint + contract evals + native (native skips without Xcode)
	$(MAKE) lint
	node --test evals/contracts.test.mjs
	bash scripts/native-gates.sh
