#!/usr/bin/env python3
"""Read-only validator for a Claude Code kit (user-level or project-level).

Checks agents, skills, rules, CLAUDE.md, settings.json, command collisions with
enabled plugins, and relative markdown links. Never writes, never touches the
network, stdlib only.

Exit codes: 0 = clean, 1 = warnings only, 2 = errors present.

Usage:
    kit_check.py                      # validate ~/.claude
    kit_check.py --root=DIR           # validate a project .claude directory
    kit_check.py --json               # machine-readable report
    kit_check.py --self-test          # run the built-in parser checks
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

FM = re.compile(r"^---\s*\n(.*?)\n---", re.S)
KEBAB = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
SECRET_NAME = re.compile(r"(API_KEY|TOKEN|SECRET|PASSWORD|CREDENTIAL)", re.I)
AGENT_KEYS = {"name", "description", "tools", "disallowedTools", "model", "memory",
              "color", "permissionMode"}
MODELS = {"opus", "sonnet", "haiku", "inherit", "fable"}
HOOK_PATH = re.compile(r"(?:python3?|bash|sh|node)\s+(~?/[^\s\"']+)")
MD_LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
SKIP_LINK = re.compile(r"^(?:[a-z][a-z0-9+.-]*:|//|#)", re.I)

findings: list[tuple[str, str, str]] = []


def add(level: str, area: str, message: str) -> None:
    findings.append((level, area, message))


def frontmatter(path: Path) -> tuple[dict[str, str], str]:
    """Parse YAML-ish frontmatter. Returns (meta, full_text).

    A key whose value is empty and whose next line starts with '- ' becomes the
    marker "(list)" -- that is how `paths:` blocks are detected.
    """
    text = path.read_text(errors="replace")
    match = FM.match(text)
    meta: dict[str, str] = {}
    if match:
        lines = match.group(1).splitlines()
        for index, line in enumerate(lines):
            if ":" not in line or line.lstrip().startswith("#"):
                continue
            key, _, value = line.partition(":")
            value = value.strip().strip("'\"")
            if not value and index + 1 < len(lines) and lines[index + 1].lstrip().startswith("- "):
                value = "(list)"
            meta.setdefault(key.strip(), value)
    return meta, text


def body_of(text: str) -> str:
    return FM.sub("", text, count=1)


def check_agents(root: Path) -> int:
    directory = root / "agents"
    if not directory.is_dir():
        return 0
    names: dict[str, Path] = {}
    for path in sorted(directory.glob("*.md")):
        meta, _ = frontmatter(path)
        name = meta.get("name", "")
        if not name:
            add("error", "agents", f"{path.name}: missing frontmatter name")
        elif not KEBAB.match(name):
            add("error", "agents", f"{path.name}: name '{name}' is not kebab-case")
        if name and name in names:
            add("error", "agents", f"duplicate agent name '{name}': {names[name].name} and {path.name}")
        names.setdefault(name, path)
        if not meta.get("description"):
            add("error", "agents", f"{path.name}: missing description")
        if meta.get("model") and meta["model"] not in MODELS and not meta["model"].startswith("claude-"):
            add("warn", "agents", f"{path.name}: unknown model '{meta['model']}'")
        if not (meta.get("tools") or meta.get("disallowedTools")):
            add("warn", "agents", f"{path.name}: inherits all tools (least privilege not declared)")
        for key in meta:
            if key not in AGENT_KEYS:
                add("warn", "agents", f"{path.name}: unknown frontmatter key '{key}'")
    return len(names)


def check_skills(root: Path) -> int:
    directory = root / "skills"
    count = 0
    if not directory.is_dir():
        return 0
    for skill in sorted(directory.glob("*/SKILL.md")):
        count += 1
        meta, text = frontmatter(skill)
        if meta.get("name") != skill.parent.name:
            # Language-kit templates carry a <app> placeholder that rename.sh
            # substitutes at scaffold time; not a defect in the template.
            level = "warn" if "<" in meta.get("name", "") else "error"
            add(level, "skills", f"{skill.parent.name}: frontmatter name != directory name")
        desc = meta.get("description", "")
        if not desc:
            add("error", "skills", f"{skill.parent.name}: missing description")
        if len(desc) + len(meta.get("when_to_use", "")) > 1536:
            add("error", "skills", f"{skill.parent.name}: description + when_to_use over 1536 chars")
        lines = len(body_of(text).splitlines())
        if lines >= 500:
            add("error", "skills", f"{skill.parent.name}: body is {lines} lines (cap 500)")
        elif lines > 150:
            add("warn", "skills", f"{skill.parent.name}: body is {lines} lines (target < 150)")
        tools = meta.get("allowed-tools", "")
        side_effects = any(tool in tools for tool in ("Write", "Edit", "Bash"))
        if side_effects and meta.get("disable-model-invocation", "").lower() != "true":
            add("warn", "skills", f"{skill.parent.name}: has side-effect tools without disable-model-invocation")
    return count


def check_rules(root: Path) -> tuple[int, int]:
    directory = root / "rules"
    files = sorted(directory.rglob("*.md")) if directory.is_dir() else []
    always = 0
    for path in files:
        meta, _ = frontmatter(path)
        if "paths" in meta and not meta["paths"].strip():
            add("error", "rules", f"{path.relative_to(directory)}: empty paths block")
        if "paths" not in meta:
            always += path.stat().st_size
    if always > 20 * 1024:
        add("warn", "rules", f"always-loaded rules are {always} bytes (target < 20480)")
    return len(files), always


def check_claude_md(root: Path) -> int:
    # A project-scope root is the .claude directory itself; its CLAUDE.md lives
    # one level up, at the repo root.
    path = root / "CLAUDE.md"
    if not path.is_file() and root.name == ".claude":
        path = root.parent / "CLAUDE.md"
    if not path.is_file():
        add("error", "claude-md", "CLAUDE.md missing")
        return 0
    lines = len(path.read_text(errors="replace").splitlines())
    if lines > 200:
        add("warn", "claude-md", f"{lines} lines (target <= 200)")
    return lines


def check_settings(root: Path) -> None:
    path = root / "settings.json"
    if not path.is_file():
        return
    try:
        settings = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        add("error", "settings", f"invalid JSON: {exc}")
        return
    for key, value in settings.get("env", {}).items():
        text = str(value)
        # A plaintext secret in settings.json is a real problem, but the key
        # name alone is a weak signal: MAX_OUTPUT_TOKENS and friends are
        # config, not credentials. Only warn, and only for long opaque values.
        if (SECRET_NAME.search(key) and text not in ("ollama", "")
                and len(text) >= 20 and not text.isdigit()):
            add("warn", "settings", f"env.{key} looks like a plaintext secret")
    for event, entries in settings.get("hooks", {}).items():
        for entry in entries:
            for hook in entry.get("hooks", []):
                for match in HOOK_PATH.finditer(hook.get("command", "")):
                    target = Path(match.group(1).replace("~", str(Path.home())))
                    if not target.exists():
                        add("error", "settings", f"{event} hook references missing file {target}")


def enabled_plugin_roots(root: Path) -> list[tuple[str, Path]]:
    """Plugin keys enabled in settings.json, mapped to their install paths."""
    try:
        settings = json.loads((root / "settings.json").read_text())
        registry = json.loads((root / "plugins" / "installed_plugins.json").read_text())["plugins"]
    except (OSError, json.JSONDecodeError, KeyError):
        return []
    enabled = {key for key, value in settings.get("enabledPlugins", {}).items() if value}
    roots: list[tuple[str, Path]] = []
    for key, entries in registry.items():
        if key not in enabled:
            continue
        for entry in entries:
            install = Path(entry["installPath"])
            if install.is_dir():
                roots.append((key, install))
    return roots


def check_command_collisions(root: Path) -> int:
    stems: set[str] = set()
    for _, install in enabled_plugin_roots(root):
        commands = install / "commands"
        if commands.is_dir():
            stems.update(path.stem for path in commands.rglob("*.md"))
    directory = root / "commands"
    if not directory.is_dir():
        return 0
    colliding = [path.stem for path in directory.glob("*.md") if path.stem in stems]
    for stem in sorted(colliding):
        add("warn", "commands", f"{stem}.md duplicates an enabled plugin command")
    return len(colliding)


def check_links(root: Path) -> int:
    """Every relative markdown link in CLAUDE.md and rules/*.md must resolve."""
    targets = [root / "CLAUDE.md"]
    rules = root / "rules"
    if rules.is_dir():
        targets.extend(sorted(rules.rglob("*.md")))
    broken = 0
    for path in targets:
        if not path.is_file():
            continue
        text = path.read_text(errors="replace")
        for match in MD_LINK.finditer(text):
            link = match.group(1).strip()
            if not link or SKIP_LINK.match(link) or link.startswith("@"):
                continue
            link = link.split("#", 1)[0]
            if not link:
                continue
            resolved = (path.parent / link).resolve()
            if not resolved.exists():
                add("warn", "links", f"{path.relative_to(root)}: broken link '{link}'")
                broken += 1
    return broken


def self_test() -> int:
    """Minimal checks for the frontmatter list handling and link resolution."""
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        (base / "rules").mkdir()
        skill = base / "rules" / "r.md"
        skill.write_text("---\npaths:\n  - '**/*.py'\nname: r\n---\nbody\n")
        meta, _ = frontmatter(skill)
        assert meta.get("paths") == "(list)", meta
        assert meta.get("name") == "r", meta
        (base / "CLAUDE.md").write_text("see [ok](rules/r.md) and [bad](rules/nope.md)\n")
        findings.clear()
        assert check_links(base) == 1, findings
        assert findings[0][0] == "warn", findings
    print("self-test: OK")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=str(Path.home() / ".claude"),
                        help="kit root to validate (default: ~/.claude)")
    parser.add_argument("--json", action="store_true", help="machine-readable report")
    parser.add_argument("--self-test", action="store_true", help="run built-in parser checks")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    root = Path(args.root).expanduser()
    if not root.is_dir():
        print(f"error: root not found: {root}", file=sys.stderr)
        return 2

    agents = check_agents(root)
    skills = check_skills(root)
    rules, always_bytes = check_rules(root)
    claude_lines = check_claude_md(root)
    check_settings(root)
    collisions = check_command_collisions(root)
    broken_links = check_links(root)
    errors = sum(1 for level, _, _ in findings if level == "error")
    warnings = sum(1 for level, _, _ in findings if level == "warn")
    summary = dict(agents=agents, skills=skills, rules=rules, always_loaded_bytes=always_bytes,
                   claude_md_lines=claude_lines, collision_warnings=collisions,
                   broken_links=broken_links, errors=errors, warnings=warnings)
    if args.json:
        print(json.dumps(dict(root=str(root), summary=summary, findings=[
            dict(level=level, area=area, message=message) for level, area, message in findings]), indent=2))
    else:
        for level, area, message in findings:
            print(f"{level.upper():5s} {area:9s} {message}")
        print(f"\nagents={agents} skills={skills} rules={rules} "
              f"always_loaded={always_bytes}B claude_md={claude_lines}L "
              f"collisions={collisions} broken_links={broken_links}")
        print(f"{errors} error(s), {warnings} warning(s)")
    return 2 if errors else 1 if warnings else 0


if __name__ == "__main__":
    sys.exit(main())
