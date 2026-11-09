"""Password reset and change types."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime


@dataclass(frozen=True)
class PasswordAccount:
    """The columns this slice needs from ``users``."""

    id: str
    email: str
    password_hash: str


@dataclass(frozen=True)
class ResetCode:
    """One password-reset attempt row."""

    id: str
    email: str
    code: str
    expires_at: datetime
    attempts: int
