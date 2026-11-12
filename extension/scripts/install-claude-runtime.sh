#!/usr/bin/env bash
#
# install-claude-runtime.sh — install the canonical .claude/{agents,skills,commands}
# layer into one or more target runtime config dirs. PURE BASH: no pnpm
# install, no external dependency. Generates per-runtime format adapters FROM
# the canonical MD+YAML at install time (TOML for codex, JSON for opencode/zed);
# hand-maintained mirror dirs are NOT committed.
#
# Idempotent: a second run reconciles (skips unchanged, updates changed). Never
# clobbers a user-owned file — a pre-existing destination is skipped with a
# recorded warning. Writes .claude/install-state.json provenance so a future
# uninstaller can reverse every operation.
#
# Usage:
#   bash scripts/install-claude-runtime.sh [--targets=a,b,...] [--dest=DIR]
#     --targets  comma-separated runtime slugs (default: claude,claude-project)
#     --dest     destination root (default: .  = current project root)
#   The manifest at .claude/manifest.json names modules + which targets each
#   applies to. claude + claude-project are always-on; every other target is
#   opt-in via --targets.
#
# Exit 0 = installed (or no-op), exit 1 = fatal error.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$ROOT_DIR/.claude/manifest.json"

DEST_ROOT="$ROOT_DIR"
TARGETS="claude,claude-project"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --targets=*) TARGETS="${1#--targets=}"; shift;;
    --dest=*)    DEST_ROOT="${1#--dest=}"; shift;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0;;
    *) echo "install-claude-runtime: unknown arg '$1'" >&2; exit 1;;
  esac
done

if [[ ! -f "$MANIFEST" ]]; then
  echo "install-claude-runtime: ERROR — manifest $MANIFEST not found" >&2
  exit 1
fi

# --- parse manifest.json (pure bash; no jq dependency) ----------------------
# Extract a JSON value for a top-level key (string or array). Best-effort: this
# installer only needs runtime_config_dirs + format_adapters + modules.*.source
# + modules.*.targets. A full JSON parser is overkill and would need a dep.
json_get_string() {
  # json_get_string '<json>' '<key>' -> first string value
  local json="$1" key="$2"
  printf '%s' "$json" | awk -v k="\"$key\"" '
    { s = s $0 }
    END {
      i = index(s, k ":")
      if (i == 0) { exit }
      rest = substr(s, i + length(k))
      # find first "
      q = index(rest, "\"")
      if (q == 0) { exit }
      rest = substr(rest, q + 1)
      e = index(rest, "\"")
      if (e == 0) { exit }
      print substr(rest, 1, e - 1)
    }
  '
}

json_array_strings() {
  # json_array_strings '<json>' '<key>' -> space-separated string items
  local json="$1" key="$2"
  printf '%s' "$json" | awk -v k="\"$key\"" '
    { s = s $0 }
    END {
      i = index(s, k ":")
      if (i == 0) { exit }
      rest = substr(s, i + length(k))
      b = index(rest, "[")
      if (b == 0) { exit }
      rest = substr(rest, b + 1)
      e = index(rest, "]")
      if (e == 0) { exit }
      arr = substr(rest, 1, e - 1)
      n = split(arr, parts, "\"")
      for (p = 2; p <= n; p += 2) print parts[p]
    }
  ' | tr '\n' ' '
}

MANIFEST_TEXT="$(cat "$MANIFEST")"

# runtime_config_dirs map (slug -> dir name)
runtime_dir() {
  local slug="$1"
  # Look for "<slug>": "<dir>" inside runtime_config_dirs.
  printf '%s' "$MANIFEST_TEXT" | awk -v s="\"$slug\"" '
    { s = s $0 }
    END {
      i = index(s, "runtime_config_dirs")
      if (i == 0) { exit }
      rest = substr(s, i)
      j = index(rest, s ":")
      if (j == 0) { exit }
      rest = substr(rest, j + length(s))
      q = index(rest, "\"")
      if (q == 0) { exit }
      rest = substr(rest, q + 1)
      e = index(rest, "\"")
      if (e == 0) { exit }
      print substr(rest, 1, e - 1)
    }
  '
}

# format_adapters map (slug -> format)
adapter_format() {
  local slug="$1"
  printf '%s' "$MANIFEST_TEXT" | awk -v s="\"$slug\"" '
    { s = s $0 }
    END {
      i = index(s, "format_adapters")
      if (i == 0) { exit }
      rest = substr(s, i)
      j = index(rest, s ":")
      if (j == 0) { exit }
      rest = substr(rest, j + length(s))
      q = index(rest, "\"")
      if (q == 0) { exit }
      rest = substr(rest, q + 1)
      e = index(rest, "\"")
      if (e == 0) { exit }
      print substr(rest, 1, e - 1)
    }
  '
}

# --- build the operation log -------------------------------------------------
declare -a OPS=()
record_op() {
  # record_op <source> <destination> <strategy> <ownership>
  OPS+=("{\"source\":\"$1\",\"destination\":\"$2\",\"strategy\":\"$3\",\"ownership\":\"$4\"}")
}

now_iso() { date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date +"%Y-%m-%dT%H:%M:%SZ"; }

mkdir -p "$DEST_ROOT/.claude"
STATE="$DEST_ROOT/.claude/install-state.json"

# Track installed_at from a prior state file (for idempotent update).
INSTALLED_AT="$(now_iso)"
if [[ -f "$STATE" ]]; then
  prev_installed="$(json_get_string "$(cat "$STATE")" "installed_at")"
  [[ -n "$prev_installed" ]] && INSTALLED_AT="$prev_installed"
fi

# --- iterate modules + targets ----------------------------------------------
IFS=',' read -ra TARGET_ARR <<< "$TARGETS"
MODULES=(agents skills commands)

copy_file() {
  # copy_file <src> <dst> — copy if dst absent or src newer; skip if user-owned.
  local src="$1" dst="$2"
  if [[ -e "$dst" && ! -L "$dst" ]]; then
    # Pre-existing, not a symlink we created -> user-owned; skip.
    record_op "$src" "$dst" "copy" "user"
    echo "install-claude-runtime: SKIP (user-owned) $dst" >&2
    return 0
  fi
  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
  record_op "$src" "$dst" "copy" "installer"
  echo "install-claude-runtime: copy $src -> $dst"
}

symlink_file() {
  local src="$1" dst="$2"
  if [[ -e "$dst" && ! -L "$dst" ]]; then
    record_op "$src" "$dst" "symlink" "user"
    echo "install-claude-runtime: SKIP (user-owned) $dst" >&2
    return 0
  fi
  mkdir -p "$(dirname "$dst")"
  ln -sfn "$src" "$dst"
  record_op "$src" "$dst" "symlink" "installer"
  echo "install-claude-runtime: symlink $src -> $dst"
}

# Adapter: convert a canonical MD+YAML skill/command/agent to a runtime format.
# This is a structural conversion (frontmatter -> native config keys), not a
# full markdown render. TOML for codex; JSON for opencode/zed.
adapt_file() {
  # adapt_file <src> <dst> <format>
  local src="$1" dst="$2" fmt="$3"
  if [[ -e "$dst" && ! -L "$dst" ]]; then
    record_op "$src" "$dst" "adapter" "user"
    echo "install-claude-runtime: SKIP (user-owned) $dst" >&2
    return 0
  fi
  mkdir -p "$(dirname "$dst")"
  local base; base="$(basename "$src")"
  local name="${base%.md}"
  local desc; desc="$(awk 'NR==1{next} /^---$/{exit} index($0,"description:"){sub("^[^:]*:[[:space:]]*",""); gsub(/["'"'"']/,""); print; exit}' "$src")"
  local div; div="$(awk 'NR==1{next} /^---$/{exit} index($0,"division:"){sub("^[^:]*:[[:space:]]*",""); gsub(/["'"'"']/,""); print; exit}' "$src")"
  case "$fmt" in
    toml)
      {
        printf 'name = "%s"\n' "$name"
        printf 'description = "%s"\n' "$desc"
        [[ -n "$div" ]] && printf 'division = "%s"\n' "$div"
        printf '\n# Adapted from %s at install time. Do not hand-edit; re-run install-claude-runtime.sh.\n' "$src"
      } > "$dst"
      ;;
    json)
      {
        printf '{\n'
        printf '  "name": "%s",\n' "$name"
        printf '  "description": "%s"' "$desc"
        [[ -n "$div" ]] && printf ',\n  "division": "%s"' "$div"
        printf '\n}\n'
      } > "$dst"
      ;;
    *)
      # Unknown format: fall back to a straight copy.
      cp "$src" "$dst"
      ;;
  esac
  record_op "$src" "$dst" "adapter" "installer"
  echo "install-claude-runtime: adapt ($fmt) $src -> $dst"
}

# install_hooks_target <dest_root> — copy scripts/hooks/*.sh into
# <dest_root>/scripts/hooks/ and .claude/settings.json into
# <dest_root>/.claude/settings.json, reusing copy_file's existing
# only-if-absent / user-owned-skip semantics (never clobbers a file the
# consumer already owns). PreToolUse hooks are a Claude-Code-native concept
# (no per-runtime format adapter makes sense), so the caller guards this to
# the claude-project target only — never cursor/codex/opencode/zed/etc.
install_hooks_target() {
  local dest_root="$1"
  local hooks_src="$ROOT_DIR/scripts/hooks"
  local settings_src="$ROOT_DIR/.claude/settings.json"
  local hooks_dest="$dest_root/scripts/hooks"
  local settings_dest="$dest_root/.claude/settings.json"

  if [[ -d "$hooks_src" ]]; then
    while IFS= read -r -d '' f; do
      local base; base="$(basename "$f")"
      copy_file "$f" "$hooks_dest/$base"
    done < <(find "$hooks_src" -maxdepth 1 -name '*.sh' -type f -print0 | sort -z)
  fi

  if [[ -f "$settings_src" ]]; then
    copy_file "$settings_src" "$settings_dest"
  fi
}

for module in "${MODULES[@]}"; do
  src_dir="$ROOT_DIR/.claude/$module"
  [[ -d "$src_dir" ]] || continue
  # module targets from manifest (best-effort parse of "modules"."<module>","targets" array)
  mod_targets_raw="$(printf '%s' "$MANIFEST_TEXT" | awk -v m="$module" '
    { s = s $0 }
    END {
      k = "\"modules\""
      i = index(s, k)
      if (i == 0) { exit }
      rest = substr(s, i)
      k2 = "\"" m "\":"
      j = index(rest, k2)
      if (j == 0) { exit }
      rest = substr(rest, j + length(k2))
      b = index(rest, "[")
      if (b == 0) { exit }
      rest = substr(rest, b + 1)
      e = index(rest, "]")
      if (e == 0) { exit }
      arr = substr(rest, 1, e - 1)
      n = split(arr, parts, "\"")
      for (p = 2; p <= n; p += 2) print parts[p]
    }
  ' | tr '\n' ' ')"
  # Intersect requested targets with module targets.
  for tgt in "${TARGET_ARR[@]}"; do
    # Always-on targets apply to every module regardless of manifest list.
    case "$tgt" in
      claude|claude-project) : ;;
      *)
        if [[ -z "$mod_targets_raw" || " $mod_targets_raw " != *"$tgt"* ]]; then
          echo "install-claude-runtime: SKIP $module for $tgt (not in manifest targets)"
          continue
        fi
        ;;
    esac
    rt_dir="$(runtime_dir "$tgt")"
    [[ -z "$rt_dir" ]] && rt_dir=".$tgt"
    fmt="$(adapter_format "$tgt")"
    dest_dir="$DEST_ROOT/$rt_dir/$module"
    mkdir -p "$dest_dir"
    # For claude + claude-project, the canonical MD IS the runtime format -> copy.
    # For adapters (codex/opencode/zed), convert. For others, copy the MD as-is.
    while IFS= read -r -d '' f; do
      rel="${f#$src_dir/}"
      dst="$dest_dir/$rel"
      case "$tgt" in
      claude|claude-project|cursor|antigravity|gemini|codebuddy|joycode|qwen|hermes|openclaw|kimi)
        copy_file "$f" "$dst"
        ;;
      codex)
        adapt_file "$f" "${dst%.md}.toml" "${fmt:-toml}"
        ;;
      opencode|zed)
        adapt_file "$f" "${dst%.md}.json" "${fmt:-json}"
        ;;
      *)
        copy_file "$f" "$dst"
        ;;
      esac
    done < <(find "$src_dir" -type f -print0 | sort -z)
  done
done

# Hooks are a Claude-Code-native concept; only the claude-project target gets
# scripts/hooks/*.sh + .claude/settings.json.
for tgt in "${TARGET_ARR[@]}"; do
  if [[ "$tgt" == "claude-project" ]]; then
    install_hooks_target "$DEST_ROOT"
  fi
done

# --- write install-state.json provenance ------------------------------------
ops_json="$(printf '%s\n' "${OPS[@]}" | grep -c '^{' )"
{
  printf '{\n'
  printf '  "schema_version": 1,\n'
  printf '  "installed_at": "%s",\n' "$INSTALLED_AT"
  printf '  "updated_at": "%s",\n' "$(now_iso)"
  printf '  "targets": [%s],\n' "$(printf '"%s",' "${TARGET_ARR[@]}" | sed 's/,$//')"
  printf '  "operations": [\n'
  first=1
  for op in "${OPS[@]}"; do
    if [[ $first -eq 1 ]]; then first=0; else printf ',\n'; fi
    printf '    %s' "$op"
  done
  printf '\n  ]\n}\n'
} > "$STATE"

echo "install-claude-runtime: wrote $STATE ($ops_json operation(s))."
echo "install-claude-runtime: done — targets=[${TARGET_ARR[*]}]."
exit 0