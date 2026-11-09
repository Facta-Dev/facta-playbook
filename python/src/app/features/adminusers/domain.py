"""Admin user-management types."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime

from app.shared.rolex import Role


@dataclass(frozen=True)
class AdminUser:
    """The admin view of one user row."""

    id: str
    email: str
    first_name: str
    last_name: str
    role: Role
    is_active: bool
    is_blocked: bool
    is_verified: bool
    language: str
    created_at: datetime
    last_login: datetime | None


@dataclass(frozen=True)
class Patch:
    """A partial update; ``None`` leaves the column untouched."""

    is_active: bool | None
    is_blocked: bool | None
    role: Role | None


@dataclass(frozen=True)
class UpgradeResult:
    """Outcome of the whitelisted bootstrap upgrade."""

    email: str
    message: str
    upgraded: bool
