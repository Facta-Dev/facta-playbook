#!/usr/bin/env python3
"""Move user agents/commands that duplicate ENABLED plugin components into a backup.

Never deletes: colliding files are moved to <backup-dir>/{agents,commands}/ and
recorded in manifest.json (sha256, size, identical flag, model/tools deltas,
note). `--restore` puts them back using the manifest's recorded paths.

Skills are reported but never moved: they are namespaced (plugin:skill) so a
same-named user skill is not a collision.

Usage:
    dedupe_plugins.py                        # dry run against ~/.claude
    dedupe_plugins.py --root=DIR             # another kit root
    dedupe_plugins.py --apply                # move into the backup dir
    dedupe_plugins.py --restore              # restore from the backup dir
    dedupe_plugins.py --rebuild-manifest     # rebuild manifest.json from the backup dir
    dedupe_plugins.py --self-test            # built-in round-trip check
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path

FM = re.compile(r"^---\s*\n(.*?)\n---", re.S)
# plural directory names, so the manifest and the restore agree with reality
DIRS = {"agent": "agents", "command": "commands"}


def frontmatter(path: Path) -> dict[str, str]:
    try:
        match = FM.match(path.read_text(errors="replace"))
    except OSError:
        return {}
    if not match:
        return {}
    out: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if ":" not in line or line.lstrip().startswith("#"):
            continue
        key, _, value = line.partition(":")
        out.setdefault(key.strip(), value.strip().strip("'\""))
    return out


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def enabled_plugin_roots(root: Path) -> list[tuple[str, Path]]:
    """Enabled plugin keys -> install paths. Missing files mean 'no plugins'."""
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


def plugin_index(root: Path) -> tuple[dict[str, tuple[str, Path]], dict[str, tuple[str, Path]], set[str]]:
    agents: dict[str, tuple[str, Path]] = {}
    commands: dict[str, tuple[str, Path]] = {}
    skills: set[str] = set()
    for key, install in enabled_plugin_roots(root):
        agent_dir = install / "agents"
        if agent_dir.is_dir():
            for path in sorted(agent_dir.glob("*.md")):
                agents.setdefault(frontmatter(path).get("name", path.stem), (key, path))
        command_dir = install / "commands"
        if command_dir.is_dir():
            for path in sorted(command_dir.rglob("*.md")):
                commands.setdefault(path.stem, (key, path))
        skill_dir = install / "skills"
        if skill_dir.is_dir():
            skills.update(path.parent.name for path in skill_dir.glob("*/SKILL.md"))
    return agents, commands, skills


def build_rows(root: Path) -> tuple[list[dict], list[dict]]:
    """(movable collisions, skill name collisions that are never moved)."""
    plugin_agents, plugin_commands, plugin_skills = plugin_index(root)
    rows: list[dict] = []
    agent_dir = root / "agents"
    if agent_dir.is_dir():
        for path in sorted(agent_dir.glob("*.md")):
            name = frontmatter(path).get("name")
            if name and name in plugin_agents:
                key, plugin_path = plugin_agents[name]
                rows.append(dict(kind="agent", src=path, name=name, plugin=key, plugin_path=plugin_path))
    command_dir = root / "commands"
    if command_dir.is_dir():
        for path in sorted(command_dir.glob("*.md")):
            if path.stem in plugin_commands:
                key, plugin_path = plugin_commands[path.stem]
                rows.append(dict(kind="command", src=path, name=path.stem, plugin=key, plugin_path=plugin_path))
    skills: list[dict] = []
    skill_dir = root / "skills"
    if skill_dir.is_dir():
        for path in sorted(skill_dir.glob("*/SKILL.md")):
            if path.parent.name in plugin_skills:
                skills.append(dict(name=path.parent.name, path=path))
    return rows, skills


def note_for(row: dict, user_meta: dict, plugin_meta: dict) -> str:
    if row["src"].read_bytes() == row["plugin_path"].read_bytes():
        return "identical to plugin copy"
    if user_meta.get("model") != plugin_meta.get("model") or user_meta.get("tools") != plugin_meta.get("tools"):
        return "differs from plugin copy (model/tools differ)"
    return "differs from plugin copy"


def entry_for(row: dict, root: Path, backup: Path) -> dict:
    user_meta = frontmatter(row["src"]) if row["kind"] == "agent" else {}
    plugin_meta = frontmatter(row["plugin_path"])
    return dict(
        kind=row["kind"], name=row["name"],
        original_path=str(root / DIRS[row["kind"]] / row["src"].name),
        backup_path=str(backup / DIRS[row["kind"]] / row["src"].name),
        sha256=sha(row["src"]), bytes=row["src"].stat().st_size,
        plugin=row["plugin"], plugin_path=str(row["plugin_path"]),
        identical=row["src"].read_bytes() == row["plugin_path"].read_bytes(),
        user_model=user_meta.get("model", ""), user_tools=user_meta.get("tools", ""),
        plugin_model=plugin_meta.get("model", ""), plugin_tools=plugin_meta.get("tools", ""),
        note=note_for(row, user_meta, plugin_meta),
    )


def write_manifest(rows: list[dict], root: Path, backup: Path) -> list[dict]:
    entries = [entry_for(row, root, backup) for row in rows]
    backup.mkdir(parents=True, exist_ok=True)
    (backup / "manifest.json").write_text(json.dumps(entries, indent=2))
    return entries


def rebuild_manifest(root: Path, backup: Path) -> int:
    """Rebuild manifest.json from the files already in the backup dir."""
    plugin_agents, plugin_commands, _ = plugin_index(root)
    rows: list[dict] = []
    for kind, index in (("agent", plugin_agents), ("command", plugin_commands)):
        directory = backup / DIRS[kind]
        if not directory.is_dir():
            continue
        for path in sorted(directory.glob("*.md")):
            name = frontmatter(path).get("name") if kind == "agent" else path.stem
            if name in index:
                key, plugin_path = index[name]
                rows.append(dict(kind=kind, src=path, name=name, plugin=key, plugin_path=plugin_path))
    entries = [entry_for(row, root, backup) for row in rows]
    backup.mkdir(parents=True, exist_ok=True)
    (backup / "manifest.json").write_text(json.dumps(entries, indent=2))
    print(f"rebuilt manifest with {len(entries)} entries -> {backup / 'manifest.json'}")
    return 0


def restore(backup: Path) -> int:
    """Restore using the manifest's recorded backup paths (no globbing)."""
    manifest = backup / "manifest.json"
    if not manifest.is_file():
        print(f"no manifest at {manifest}")
        return 0
    entries = json.loads(manifest.read_text())
    moved = 0
    for entry in entries:
        source = Path(entry["backup_path"])
        target = Path(entry["original_path"])
        if not source.exists():
            print(f"SKIP (missing in backup) {source}")
            continue
        if target.exists():
            print(f"SKIP (exists) {target}")
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(source), str(target))
        moved += 1
    print(f"restored {moved} files")
    return 0


def self_test() -> int:
    """Round trip: move a colliding agent into the backup, restore it back."""
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        root = base / "kit"
        plugin = base / "plugin"
        (root / "agents").mkdir(parents=True)
        (plugin / "agents").mkdir(parents=True)
        body = "---\nname: dup\ndescription: d\n---\nhi\n"
        (root / "agents" / "dup.md").write_text(body)
        (plugin / "agents" / "dup.md").write_text(body)
        (root / "settings.json").write_text(json.dumps({"enabledPlugins": {"p@v": True}}))
        (root / "plugins").mkdir()
        (root / "plugins" / "installed_plugins.json").write_text(
            json.dumps({"plugins": {"p@v": [{"installPath": str(plugin)}]}}))
        backup = base / "backup"
        rows, _ = build_rows(root)
        assert len(rows) == 1, rows
        entries = write_manifest(rows, root, backup)
        assert entries[0]["identical"] is True, entries
        target = backup / "agents" / "dup.md"
        assert entries[0]["backup_path"].endswith("/agents/dup.md"), entries[0]["backup_path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(root / "agents" / "dup.md"), str(target))
        assert restore(backup) == 0
        assert (root / "agents" / "dup.md").is_file(), "restore must put the file back"
    print("self-test: OK")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=str(Path.home() / ".claude"),
                        help="kit root (default: ~/.claude)")
    parser.add_argument("--backup-dir", default=None,
                        help="backup destination (default: <root>/backups/dedupe-<UTC date>)")
    parser.add_argument("--apply", action="store_true", help="move colliding files into the backup dir")
    parser.add_argument("--restore", action="store_true", help="restore files from the backup dir")
    parser.add_argument("--rebuild-manifest", action="store_true", help="rebuild manifest.json from the backup dir")
    parser.add_argument("--dry-run", action="store_true", help="report only (default)")
    parser.add_argument("--self-test", action="store_true", help="run the built-in round-trip check")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    root = Path(args.root).expanduser()
    backup = Path(args.backup_dir).expanduser() if args.backup_dir else \
        root / "backups" / f"dedupe-{dt.datetime.now(dt.timezone.utc):%Y-%m-%d}"

    if args.restore:
        return restore(backup)
    if args.rebuild_manifest:
        return rebuild_manifest(root, backup)

    rows, skills = build_rows(root)
    if not enabled_plugin_roots(root):
        print("no plugins: nothing to dedupe")
        return 0
    agents = sum(1 for row in rows if row["kind"] == "agent")
    commands = sum(1 for row in rows if row["kind"] == "command")
    print(f"collisions: {agents} agents, {commands} commands")
    for row in rows:
        same = "identical" if row["src"].read_bytes() == row["plugin_path"].read_bytes() else "DIFFERS"
        print(f"  move    {row['kind']:7s} {row['src'].name:40s} -> {row['plugin']} [{same}]")
    for skill in skills:
        print(f"  report  skill    {skill['name']:40s} -> namespaced, not moved")
    if not args.apply:
        print("\ndry run; re-run with --apply to move into the backup root")
        return 0

    entries = write_manifest(rows, root, backup)
    for row in rows:
        target = backup / DIRS[row["kind"]] / row["src"].name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(row["src"]), str(target))
    print(f"moved {len(entries)} files; manifest -> {backup / 'manifest.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
