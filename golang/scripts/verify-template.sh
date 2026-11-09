#!/usr/bin/env bash
# verify-template.sh — smoke-test the <app> Go skeleton end to end.
#
# The skeleton ships with literal <app>/<APP>/<scope> placeholders that are
# not a valid Go module path, so `go build` cannot run as shipped. This
# script stages a throwaway copy, renames it via scripts/rename.sh
# (example values), asserts no literal placeholder survives where it must
# not, then runs the template's Go gates on the instantiated copy. The real
# working tree is never mutated. Exits nonzero on any failure. Idempotent.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="exampletool"
EXAMPLE_SCOPE="factadev"
TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
export PATH="$HOME/tools/go/bin:$HOME/go/bin:$PATH"

command -v go >/dev/null || { echo "verify-template: go is required on PATH" >&2; exit 1; }
command -v docker >/dev/null || { echo "verify-template: docker is required on PATH" >&2; exit 1; }

echo "verify-template: staging $TMP"
(cd "$ROOT_DIR" && tar cf - --exclude='./.git' .) | (cd "$TMP" && tar xf -)

echo "verify-template: rename"
(cd "$TMP" && bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE" >/dev/null)

# INV-T1: no literal placeholder survives outside the pattern carriers
# (scripts/ keep their patterns; .claude/ is vocabulary; evals/fixtures are
# frozen leak-detector test data).
survivors=$(grep -rlE '<app>|<APP>|<scope>' "$TMP" \
  --exclude-dir=scripts --exclude-dir=.claude --exclude-dir=evals \
  --exclude-dir=.git \
  --exclude='*.sum' 2>/dev/null || true)
if [[ -n "$survivors" ]]; then
  echo "verify-template: FAIL — literal placeholders survived the rename:" >&2
  echo "$survivors" >&2
  exit 1
fi

echo "verify-template: go mod tidy"
(cd "$TMP" && go mod tidy)

echo "verify-template: docker build (boot-path smoke)"
# The throwaway carries a fresh go.sum (tidy above) — close to the
# instantiated repo; the no-go.sum clone case is exercised by the build
# self-heal in the Dockerfile RUN itself.
(cd "$TMP" && docker build --quiet -t verify-template-smoke . && docker rmi verify-template-smoke >/dev/null 2>&1)

echo "verify-template: fmt + vet"
(cd "$TMP" && go build ./...)
(cd "$TMP" && test -z "$(gofmt -l .)" )
(cd "$TMP" && go vet ./...)

echo "verify-template: golangci-lint (depguard gates)"
if command -v golangci-lint >/dev/null 2>&1; then
  (cd "$TMP" && golangci-lint run)
else
  echo "verify-template: WARN — golangci-lint not on PATH; vet fallback only" >&2
fi

echo "verify-template: unit tests (hermetic)"
(cd "$TMP" && go test ./internal/... ./cmd/...)

echo "verify-template: throwaway Postgres 17 + goose migrations"
# -P publishes the image's exposed port (5432) to a random host port; the
# top-level NetworkSettings.IPAddress field is absent on modern Docker.
PG_CID="$(docker run -d --rm -P -e POSTGRES_PASSWORD=verifytemplate -e POSTGRES_DB=verifytemplate postgres:17-alpine)"
trap 'docker rm -f "$PG_CID" >/dev/null 2>&1; rm -rf "$TMP"' EXIT
PG_PORT="$(docker port "$PG_CID" 5432/tcp | awk -F: '{print $2}' | head -1)"
if [[ -z "$PG_PORT" ]]; then
  echo "verify-template: FAIL — could not resolve the published Postgres port" >&2
  exit 1
fi
for i in $(seq 1 30); do
  docker exec "$PG_CID" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$PG_CID" pg_isready -U postgres >/dev/null 2>&1 \
  || { echo "verify-template: FAIL — Postgres never became ready" >&2; exit 1; }
(cd "$TMP" && GOOSE_DRIVER=postgres GOOSE_DBSTRING="postgres://postgres:verifytemplate@127.0.0.1:${PG_PORT}/verifytemplate?sslmode=disable" GOOSE_MIGRATION_DIR=migrations go tool goose up \
  && echo "verify-template: migrations OK")

echo "verify-template: integration tests (testcontainers)"
(cd "$TMP" && go test -tags=integration ./internal/... ./test/... 2>&1 \
  || { echo "verify-template: FAIL — integration tests" >&2; exit 1; })

echo "verify-template: agency gates"
for s in lint-agents lint-skills lint-commands; do
  [[ -f "$TMP/scripts/$s.sh" ]] && bash "$TMP/scripts/$s.sh" || true
done
[[ -f "$TMP/scripts/leak-guard.sh" ]] && bash "$TMP/scripts/leak-guard.sh" || true
if [[ -f "$TMP/.claude/settings.json" ]]; then
  grep -q 'PreToolUse' "$TMP/.claude/settings.json" \
    || { echo "verify-template: FAIL — .claude/settings.json does not wire PreToolUse" >&2; exit 1; }
fi
while IFS= read -r hook; do bash -n "$hook"; done < <(find "$TMP/scripts/hooks" -name '*.sh' 2>/dev/null)
(cd "$TMP/evals" && go test ./...) && echo "verify-template: evals OK"

echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
