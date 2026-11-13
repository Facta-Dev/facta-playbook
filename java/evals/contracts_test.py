#!/usr/bin/env python3
"""Contract suite for the <app> .claude layer (stdlib only)."""
from __future__ import annotations
import re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLAUDE = ROOT / ".claude"
ERRORS = []

def field(text, key):
    m = re.search(rf"^{key}:\s*(.+)$", text, re.M)
    return m.group(1).strip().strip('"') if m else None

def sections(text):
    return set(re.findall(r"^##\s+(.+)$", text, re.M))

def check_agent(path, required_fields, required_sections, divisions, strict=True, strict_dir=True):
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        ERRORS.append(f"{path}: missing frontmatter"); return
    for key in required_fields:
        if not field(text, key): ERRORS.append(f"{path}: missing {key}")
    division = field(text, "division")
    if strict and division and division not in divisions:
        ERRORS.append(f"{path}: bad division {division}")
    if strict and strict_dir and division and path.parent.name != division:
        ERRORS.append(f"{path}: directory/division mismatch")
    for section in required_sections:
        if not any(s.lower() == section.lower() for s in sections(text)):
            ERRORS.append(f"{path}: missing section {section}")

def main():
    schema = (CLAUDE / "schemas/agents.schema.md").read_text(encoding="utf-8")
    def items(heading):
        out, on = [], False
        for line in schema.splitlines():
            if line.startswith("## "): on = line == f"## {heading}"
            elif on and line.startswith("- "): out.append(line[2:].strip())
        return out
    agent_fields = items("Required Frontmatter Fields")
    agent_sections = items("Required Body Sections")
    divisions = items("Allowed Divisions")
    for path in sorted((CLAUDE / "agents").rglob("*.md")):
        check_agent(path, agent_fields, agent_sections, divisions)
    skills_schema = (CLAUDE / "schemas/skills.schema.md").read_text(encoding="utf-8")
    def sitems(heading):
        out, on = [], False
        for line in skills_schema.splitlines():
            if line.startswith("## "): on = line == f"## {heading}"
            elif on and line.startswith("- "): out.append(line[2:].strip())
        return out
    skill_fields = sitems("Required Frontmatter Fields")
    skill_sections = sitems("Required Body Sections")
    for path in sorted((CLAUDE / "skills").glob("*/SKILL.md")):
        check_agent(path, skill_fields, skill_sections, divisions, strict_dir=False)
    for path in sorted((CLAUDE / "commands").glob("*.md")):
        text = path.read_text(encoding="utf-8")
        for key in ("name", "description", "division"):
            if not field(text, key): ERRORS.append(f"{path}: missing {key}")
        if "## Delegate" not in text: ERRORS.append(f"{path}: missing Delegate")
    if "PreToolUse" not in (CLAUDE / "settings.json").read_text(encoding="utf-8"):
        ERRORS.append(".claude/settings.json: missing PreToolUse")
    if not (CLAUDE / "manifest.json").is_file():
        ERRORS.append(".claude/manifest.json: missing")
    # Frozen fixtures: the valid pair passes, the invalid pair fails.
    def fixture_errors(path, fields, required, strict_dir):
        before = len(ERRORS)
        check_agent(path, fields, required, divisions, strict_dir=strict_dir)
        found = len(ERRORS) - before
        del ERRORS[before:]
        return found
    if fixture_errors(ROOT / "evals/fixtures/valid/agent.md", agent_fields, agent_sections, False) != 0:
        ERRORS.append("valid agent fixture unexpectedly invalid")
    if fixture_errors(ROOT / "evals/fixtures/invalid/agent.md", agent_fields, agent_sections, False) == 0:
        ERRORS.append("invalid agent fixture unexpectedly valid")
    if fixture_errors(ROOT / "evals/fixtures/valid/skill.md", skill_fields, skill_sections, False) != 0:
        ERRORS.append("valid skill fixture unexpectedly invalid")
    if fixture_errors(ROOT / "evals/fixtures/invalid/skill.md", skill_fields, skill_sections, False) == 0:
        ERRORS.append("invalid skill fixture unexpectedly valid")
    if ERRORS:
        print("\n".join(ERRORS)); print(f"evals: FAIL ({len(ERRORS)} errors)"); return 1
    print("evals: OK"); return 0

if __name__ == "__main__":
    sys.exit(main())
