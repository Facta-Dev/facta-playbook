# <app> — build and verification targets.
MVN ?= mvn
PYTHON ?= python3

.DEFAULT_GOAL := help
.PHONY: help build test fmt-check ci evals verify run migrate package

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-14s %s\n",$$1,$$2}'

build: ## Compile and package the WAR
	$(MVN) -q -DskipTests package

package: build

test: ## Unit + ArchUnit tests
	$(MVN) -q test

fmt-check: ## Checkstyle
	$(MVN) -q checkstyle:check

ci: ## Checkstyle + tests
	$(MAKE) fmt-check
	$(MAKE) test

evals: ## .claude contract suite
	$(PYTHON) evals/contracts_test.py

verify: ## Staged renamed copy + every gate
	bash scripts/verify-template.sh

run: ## Run on a local application server (deploy target/app.war)
	@echo "Deploy target/app.war to WildFly, Payara or Open Liberty."

migrate: ## Apply Flyway migrations with the DATABASE_URL
	$(MVN) -q flyway:migrate
