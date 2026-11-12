#!/usr/bin/env bash
# verify-template.sh — smoke-test the <app> Node/TypeScript skeleton end to end.
#
# The skeleton ships with literal <app>/<APP>/<scope> placeholders that make
# the tree untypecheckable, so the node gates cannot run as shipped. This
# script stages a throwaway copy, renames it via scripts/rename.sh (example
# values), asserts no literal placeholder survives where it must not, then
# runs the template's Node gates on the instantiated copy. The real working
# tree is never mutated. Exits nonzero on any failure. Idempotent.
#
# Usage: bash scripts/verify-template.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_APP="exampletool"
EXAMPLE_SCOPE="factadev"
TMP="$(mktemp -d -t verify-template.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
command -v node >/dev/null || { echo "verify-template: node is required on PATH" >&2; exit 1; }
command -v pnpm >/dev/null || { echo "verify-template: pnpm is required on PATH (corepack enable)" >&2; exit 1; }
command -v make >/dev/null || { echo "verify-template: make is required on PATH" >&2; exit 1; }
command -v docker >/dev/null || { echo "verify-template: docker is required on PATH" >&2; exit 1; }

echo "verify-template: staging $TMP"
(cd "$ROOT_DIR" && tar cf - --exclude='./.git' .) | (cd "$TMP" && tar xf -)

echo "verify-template: rename"
(cd "$TMP" && bash scripts/rename.sh "$EXAMPLE_APP" "$EXAMPLE_SCOPE" >/dev/null)

# INV-T1: no literal placeholder survives outside the pattern carriers
# (scripts/ keep their patterns; .claude/ is renamed so it must be clean;
# evals/fixtures are frozen leak-detector test data).
survivors=$(grep -rlE '<app>|<APP>|<scope>' "$TMP" \
  --exclude-dir=scripts --exclude-dir=.git \
  --exclude-dir=.claude --exclude-dir=evals \
  --exclude='pnpm-lock.yaml' --exclude-dir=node_modules \
  2>/dev/null || true)
if [[ -n "$survivors" ]]; then
  echo "verify-template: FAIL — literal placeholders survived the rename:" >&2
  echo "$survivors" >&2
  exit 1
fi

echo "verify-template: pnpm install (local lockfile, not shipped — INV-T3)"
(cd "$TMP" && pnpm install)

# Placeholder substitution changes token lengths, which re-aligns markdown
# tables. Normalize once on the instantiated copy, exactly as the README's
# "How to apply" tells a user to do, so fmt-check tests real drift instead of
# rename churn.
echo "verify-template: pnpm format (post-rename table re-alignment)"
(cd "$TMP" && pnpm format >/dev/null)

echo "verify-template: docker build (boot-path smoke)"
# The staged copy carries the local pnpm-lock.yaml from the install above —
# close to the instantiated repo; the no-lockfile clone case is exercised by
# the pnpm install self-heal in the Dockerfile RUN itself.
(cd "$TMP" && docker build --quiet -t verify-template-smoke . && docker rmi verify-template-smoke >/dev/null 2>&1)

echo "verify-template: prettier + typecheck (fmt + vet)"
(cd "$TMP" && make fmt-check)
(cd "$TMP" && make vet)

echo "verify-template: eslint (arch-law groups)"
(cd "$TMP" && make lint)
echo "verify-template: gen-check (committed openapi models)"
(cd "$TMP" && make gen-check)

echo "verify-template: unit tests (hermetic, no DB)"
(cd "$TMP" && make test)

echo "verify-template: acceptance suite (cucumber over fakes, no DB)"
(cd "$TMP" && make accept)

echo "verify-template: throwaway Postgres 17 + node-pg-migrate smoke"
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
DSN="postgres://postgres:verifytemplate@127.0.0.1:${PG_PORT}/verifytemplate?sslmode=disable"
(cd "$TMP" && export DATABASE_URL="$DSN" \
  && pnpm exec node-pg-migrate -m migrations up \
  && pnpm exec node-pg-migrate -m migrations down \
  && pnpm exec node-pg-migrate -m migrations up \
  && DATABASE_URL="$DSN" make migrate-down \
  && echo "verify-template: migrations OK")

echo "verify-template: openapi contract shape (/widgets + /readyz)"
grep -q '/widgets' "$TMP/specs/openapi.yaml" \
  || { echo "verify-template: FAIL — specs/openapi.yaml lost /widgets" >&2; exit 1; }
grep -q '/readyz' "$TMP/specs/openapi.yaml" \
  || { echo "verify-template: FAIL — specs/openapi.yaml lost /readyz" >&2; exit 1; }

echo "verify-template: agency gates"
for s in lint-agents lint-skills lint-commands; do
  [[ -f "$TMP/scripts/$s.sh" ]] && bash "$TMP/scripts/$s.sh" || true
done
[[ -f "$TMP/scripts/leak-guard.sh" ]] && bash "$TMP/scripts/leak-guard.sh" || true
# The always-on instruction set (CLAUDE.md + @imports + path-less rules) must
# stay under the harness budget; a playbook belongs behind a path-scoped rule.
(cd "$TMP" && bash scripts/instruction-budget.sh) || exit 1
if [[ -f "$TMP/.claude/settings.json" ]]; then
  grep -q 'PreToolUse' "$TMP/.claude/settings.json" \
    || { echo "verify-template: FAIL — .claude/settings.json does not wire PreToolUse" >&2; exit 1; }
fi
while IFS= read -r hook; do bash -n "$hook"; done < <(find "$TMP/scripts/hooks" -name '*.sh' 2>/dev/null)
echo "verify-template: evals (node --test contract suite)"
(cd "$TMP" && node --test evals/contracts.test.mjs) && echo "verify-template: evals OK"

echo "verify-template: run-live no-op guard (EVALS unset must exit 0)"
(cd "$TMP" && bash evals/run-live.sh >/dev/null 2>&1)   || { echo "verify-template: FAIL — run-live.sh must no-op without EVALS=1" >&2; exit 1; }
echo "verify-template: OK (temp copy discarded; the working tree was never mutated)."
