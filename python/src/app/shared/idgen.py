"""Identifier and token generation, isolated so tests can inject a source."""

from __future__ import annotations

import secrets
import uuid
from typing import Protocol


class IdSource(Protocol):
    """Everything the application needs to mint identifiers."""

    def new_id(self) -> str:
        """Opaque primary key."""
        ...

    def opaque_token(self) -> str:
        """128-character refresh token."""
        ...

    def numeric_code(self) -> str:
        """Six-digit email code, zero-padded."""
        ...


class SystemIds:
    """Production source backed by ``secrets`` and ``uuid4``."""

    def new_id(self) -> str:
        return str(uuid.uuid4())

    def opaque_token(self) -> str:
        return secrets.token_hex(64)

    def numeric_code(self) -> str:
        return f"{secrets.randbelow(1_000_000):06d}"
