# <app> — Qt 6 desktop build and verification targets.
#
# The presets pin the generator and the cache; the Makefile wires them to the
# targets below so a fresh clone needs one command: `make ci`.

CMAKE ?= cmake
CTEST ?= ctest
NINJA := $(shell command -v ninja 2>/dev/null)
PRESET ?= dev
JOBS ?= $(shell nproc 2>/dev/null || echo 4)

# Empty means "whatever the preset decides" (dev: on, release: off).
ENABLE_TESTS ?=

ifeq ($(PRESET),release)
BUILD_DIR ?= build-release
CONFIG ?= Release
else
BUILD_DIR ?= build
CONFIG ?= Debug
endif
EVALS_BUILD_DIR ?= build-evals

.DEFAULT_GOAL := help
.PHONY: help configure build test lint tidy evals verify ci

help: ## Show targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "};{printf "  %-10s %s\n",$$1,$$2}'

configure: ## Configure the build tree (Ninja when available)
	@tests=""; \
	if [ -n "$(ENABLE_TESTS)" ]; then tests="-DENABLE_TESTS=$(ENABLE_TESTS)"; fi; \
	if [ -f "$(BUILD_DIR)/CMakeCache.txt" ]; then \
	  $(CMAKE) -S . -B $(BUILD_DIR) -DCMAKE_BUILD_TYPE=$(CONFIG) $$tests; \
	else \
	  $(CMAKE) --preset $(PRESET) $$tests; \
	fi

build: configure ## Compile everything
	$(CMAKE) --build $(BUILD_DIR) -j $(JOBS)

test: ## Build with the test suite enabled and run it headless
	$(MAKE) build ENABLE_TESTS=ON
	$(CTEST) --test-dir $(BUILD_DIR) --output-on-failure

lint: ## Fail on unformatted sources (clang-format)
	@if command -v clang-format >/dev/null 2>&1; then \
	  files=$$(find src packages tests evals -type f \( -name '*.cpp' -o -name '*.h' \) 2>/dev/null); \
	  if [ -n "$$files" ]; then clang-format --dry-run -Werror $$files; fi; \
	else \
	  echo "clang-format not found — format check skipped"; \
	fi

tidy: ## Static analysis with clang-tidy (skipped without the toolchain)
	@if command -v clang-tidy >/dev/null 2>&1 && [ -f "$(BUILD_DIR)/compile_commands.json" ]; then \
	  clang-tidy -p $(BUILD_DIR) $$(find src packages -type f -name '*.cpp' 2>/dev/null); \
	else \
	  echo "clang-tidy or compile_commands.json missing — static analysis skipped"; \
	fi

evals: ## Build the evals harness (EVALS=ON) and run its checks
	@gen=""; \
	if [ ! -f "$(EVALS_BUILD_DIR)/CMakeCache.txt" ] && [ -n "$(NINJA)" ]; then gen="-G Ninja"; fi; \
	$(CMAKE) -S . -B $(EVALS_BUILD_DIR) $$gen -DCMAKE_BUILD_TYPE=$(CONFIG) \
	  -DEVALS=ON -DENABLE_TESTS=ON
	$(CMAKE) --build $(EVALS_BUILD_DIR) -j $(JOBS)
	$(CTEST) --test-dir $(EVALS_BUILD_DIR) --output-on-failure

verify: test ## Configure, build and run the tests

ci: lint verify ## The gate: format check, build and tests
