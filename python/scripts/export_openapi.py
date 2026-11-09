"""Export the OpenAPI schema from the app; the contract cannot drift.

``python scripts/export_openapi.py`` rewrites ``specs/openapi.yaml``.
``python scripts/export_openapi.py --check`` fails when the committed file is
stale, which is what CI runs.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import yaml

from app.bin.api import build_app, wire
from app.platform.config import Settings

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "specs" / "openapi.yaml"


def schema() -> dict[str, Any]:
    """Build the app without opening a socket and return its contract."""
    settings = Settings(jwt_secret="example-jwt-secret-0123456789abcdef", offline=True)
    return build_app(wire(settings)).openapi()


def render() -> str:
    return yaml.safe_dump(schema(), sort_keys=True, allow_unicode=True)


def main(argv: list[str]) -> int:
    wanted = render()
    if "--check" in argv:
        current = TARGET.read_text(encoding="utf-8") if TARGET.exists() else ""
        if current != wanted:
            print("specs/openapi.yaml is stale — run `make generate` and commit")
            return 1
        return 0
    TARGET.write_text(wanted, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
