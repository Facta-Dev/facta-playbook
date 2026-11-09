"""Contract validator + tests for the .claude layer (INV-T16: local pytest).

Hand-rolled frontmatter parser plus a body-section checker for agents,
skills and commands. The required fields, sections, allowed divisions and
failure modes are hardcoded here; the schema docs are the source of truth and
the bash linters (``scripts/lint-*.sh``) parse those docs at runtime, so
linter<->schema drift is caught by the linters. When a list changes in a
schema doc, update the matching static list below.

Fixtures are frozen test data: ``rename.sh`` excludes ``evals/fixtures/`` from
its substitution pass, so the invalid leak fixture keeps its literal tokens
and keeps exercising the leak detector.

This file carries no literal template substitution token: the leak-detector
tokens are assembled at runtime in ``PLACEHOLDER_TOKENS``.
"""

from __future__ import annotations

import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

# The three template substitution tokens, assembled from pieces so this file
# never spells one out literally.
PLACEHOLDER_TOKENS = ["<" + "app>", "<" + "APP>", "<" + "scope>"]

KIND_AGENT = "agent"
KIND_SKILL = "skill"
KIND_COMMAND = "command"

ALLOWED_DIVISIONS = ["backend", "security", "testing", "core"]


def required_frontmatter(kind: str) -> list[str]:
    if kind == KIND_AGENT:
        return ["name", "description", "color", "division"]
    return ["name", "description", "division"]


def required_sections(kind: str) -> list[str]:
    if kind == KIND_AGENT:
        return [
            "Core Mission",
            "Expertise Areas",
            "Output Format",
            "Quality Standards",
            "Guiding Principles",
        ]
    if kind == KIND_SKILL:
        return [
            "When to Activate",
            "Core Principles",
            "Rules",
            "Research References",
            "Output Format",
        ]
    return ["Delegate"]


def parse_frontmatter(text: str) -> tuple[list[tuple[str, str]], str, bool]:
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        return [], text, False
    pairs: list[tuple[str, str]] = []
    body: list[str] = []
    closed = False
    for line in lines[1:]:
        if not closed:
            if line.strip() == "---":
                closed = True
                continue
            stripped = line.strip()
            if stripped == "" or stripped.startswith("#"):
                continue
            key, sep, value = line.partition(":")
            if sep:
                value = value.strip()
                if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
                    value = value[1:-1]
                pairs.append((key.strip(), value))
            continue
        body.append(line)
    return pairs, "\n".join(body), closed


def has_section(body: str, section: str) -> bool:
    needle = section.lower()
    for line in body.split("\n"):
        stripped = line.lstrip(" \t")
        if not stripped.startswith("#"):
            continue
        heading = stripped.lstrip("#").strip().lower()
        if heading == needle:
            return True
    return False


def has_placeholder_leak(text: str) -> bool:
    return any(token in text for token in PLACEHOLDER_TOKENS)


def validate(text: str, kind: str) -> list[str]:
    errors: list[str] = []
    if "\r" in text:
        errors.append("CRLF line endings — convert to LF")
    pairs, body, closed = parse_frontmatter(text)
    if not closed:
        errors.append("missing or malformed frontmatter (no closing ---)")
        return errors
    if not pairs:
        errors.append("empty frontmatter")
    keys = {key for key, _ in pairs}
    for field in required_frontmatter(kind):
        if field not in keys:
            errors.append(f"missing frontmatter key '{field}'")
    division = next((value for key, value in pairs if key == "division"), "")
    if "division" in keys and division not in ALLOWED_DIVISIONS:
        errors.append(f"bad division '{division}'")
    for section in required_sections(kind):
        if not has_section(body, section):
            errors.append(f"missing body section '{section}'")
    if has_placeholder_leak(text):
        joined = "/".join(PLACEHOLDER_TOKENS)
        errors.append(
            f"literal placeholder leak ({joined}) — rename.sh must substitute all placeholders"
        )
    return errors


_IMPORT_RE = re.compile(r"^\s*(?:from\s+([\w.]+)\s+import|import\s+([\w.]+))")
_FORBIDDEN_ROOTS = ("fastapi", "sqlalchemy", "asyncpg", "alembic", "httpx", "uvicorn")


def domain_import_violations(text: str) -> list[str]:
    """Flag adapter, platform and cross-slice imports in a domain module."""
    violations: list[str] = []
    for raw in text.split("\n"):
        line = raw.split("#", 1)[0]
        match = _IMPORT_RE.match(line)
        if match is None:
            continue
        spec = match.group(1) or match.group(2)
        root = spec.split(".")[0]
        if root in _FORBIDDEN_ROOTS:
            violations.append(
                f"forbidden import '{spec}' — domain is pure; move I/O to http.py/postgres.py"
            )
        elif spec.startswith("app.platform"):
            violations.append(
                f"forbidden import '{spec}' — domain depends on its own ports, not platform"
            )
        elif spec.startswith("app.bin"):
            violations.append(f"forbidden import '{spec}' — nothing depends on the root")
        elif spec.startswith("app.features."):
            violations.append(f"forbidden import '{spec}' — slices never depend on other slices")
    return violations


def read_fixture(rel: str) -> str:
    return (HERE / rel).read_text(encoding="utf-8")


def renamed_reported() -> bool:
    """True once scripts/rename.sh has rewritten the project name."""
    pyproject = (ROOT / "pyproject.toml").read_text(encoding="utf-8")
    return 'name = "app-template"' not in pyproject


# --- tests -----------------------------------------------------------------


def test_valid_skill_fixture_passes() -> None:
    assert validate(read_fixture("fixtures/valid/skill.md"), KIND_SKILL) == []


def test_valid_agent_fixture_passes() -> None:
    assert validate(read_fixture("fixtures/valid/agent.md"), KIND_AGENT) == []


def test_valid_command_fixture_passes() -> None:
    assert validate(read_fixture("fixtures/valid/command.md"), KIND_COMMAND) == []


def test_missing_frontmatter_key_fails() -> None:
    errors = validate(read_fixture("fixtures/invalid/missing-frontmatter-key.md"), KIND_SKILL)
    assert any("missing frontmatter key 'division'" in err for err in errors)


def test_missing_body_section_fails() -> None:
    errors = validate(read_fixture("fixtures/invalid/missing-body-section.md"), KIND_SKILL)
    assert any("missing body section 'Output Format'" in err for err in errors)


def test_literal_placeholder_leak_fails() -> None:
    errors = validate(read_fixture("fixtures/invalid/literal-placeholder-leak.md"), KIND_SKILL)
    assert any("literal placeholder leak" in err for err in errors)


def test_bad_slice_import_flags_fastapi() -> None:
    violations = domain_import_violations(read_fixture("fixtures/invalid/bad-slice-import.py.txt"))
    assert violations
    assert any("forbidden import 'fastapi'" in item for item in violations)


def test_bad_division_fails() -> None:
    text = '---\nname: "s"\ndescription: "d"\ndivision: frontend\n---\n\n## When to Activate\n\nx'
    errors = validate(text, KIND_SKILL)
    assert any("bad division 'frontend'" in err for err in errors)


def test_empty_frontmatter_fails() -> None:
    errors = validate("---\n---\n\n# body\n", KIND_SKILL)
    assert any("empty frontmatter" in err for err in errors)


def test_no_frontmatter_fails() -> None:
    errors = validate("# No frontmatter here\n\n## When to Activate\n", KIND_SKILL)
    assert any("missing or malformed frontmatter" in err for err in errors)


def test_crlf_line_endings_fail() -> None:
    text = read_fixture("fixtures/valid/skill.md").replace("\n", "\r\n")
    errors = validate(text, KIND_SKILL)
    assert any("CRLF line endings" in err for err in errors)


def test_pure_domain_imports_pass() -> None:
    text = (
        "from __future__ import annotations\n"
        "from app.shared.apperr import AppError\n"
        "from .domain import Widget\n"
    )
    assert domain_import_violations(text) == []


def test_cross_slice_import_flagged() -> None:
    text = "from app.features.authsignup.domain import SignupUser\n"
    assert domain_import_violations(text)


def test_no_placeholder_leaks_outside_agency_layer() -> None:
    if not renamed_reported():
        return  # pre-rename: placeholders outside the agency layer are expected
    skip_dirs = {
        ".git",
        "scripts",
        "evals",
        "fixtures",
        ".venv",
        "__pycache__",
        "build",
        "dist",
        ".mypy_cache",
        ".ruff_cache",
        ".pytest_cache",
        ".import_linter_cache",
    }
    binary_ext = {".png", ".jpg", ".jpeg", ".ico", ".gif", ".woff", ".woff2", ".ttf", ".otf"}
    leaks: list[str] = []
    for path in ROOT.rglob("*"):
        if not path.is_file() or path.suffix.lower() in binary_ext:
            continue
        relative = path.relative_to(ROOT)
        if any(part in skip_dirs for part in relative.parts):
            continue
        if path.name == "uv.lock":
            continue
        data = path.read_bytes()
        if b"\x00" in data:
            continue
        text = data.decode("utf-8", errors="ignore")
        if has_placeholder_leak(text):
            leaks.append(str(relative))
    assert leaks == [], "literal placeholder leak outside the agency layer: " + ", ".join(leaks)
