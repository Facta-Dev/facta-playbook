# <app> — Android build and verification targets.
#
# The Gradle wrapper pins the Gradle version, so a fresh clone needs one
# command: `make ci`. A full JDK (17+) and an Android SDK with platform 37
# are the only machine prerequisites.

GRADLE   ?= ./gradlew
JOBS     ?= $(shell nproc 2>/dev/null || echo 4)

# AGP compiles with javac from the JDK that runs Gradle. A JRE-only default on
# PATH fails the build with "does not provide the required capabilities:
# [JAVA_COMPILER]", so pin a JDK that has both binaries before the build.
JAVA_HOME := $(shell bash scripts/jdk.sh)
export JAVA_HOME

.DEFAULT_GOAL := help
.PHONY: help test evals build lint clean verify ci

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-10s %s\n",$$1,$$2}'

test: ## Run the pure JVM and the Android unit tests
	$(GRADLE) test testDebugUnitTest

evals: ## Run the .claude contract tests
	$(GRADLE) :evals:test

build: ## Assemble the debug APK
	$(GRADLE) :app:assembleDebug

lint: ## Run Android Lint on the debug variant
	$(GRADLE) :app:lintDebug

clean: ## Remove the build output
	$(GRADLE) clean

verify: ## Stage a renamed copy and run every gate on it
	bash scripts/verify-template.sh

ci: ## The gate: architecture + lint + tests + debug build
	bash scripts/check-architecture.sh
	$(GRADLE) test testDebugUnitTest :app:lintDebug :app:assembleDebug
