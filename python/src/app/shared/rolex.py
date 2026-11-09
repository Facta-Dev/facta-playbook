"""Roles and their total order."""

from __future__ import annotations

from enum import StrEnum


class Role(StrEnum):
    """Account roles, lowest privilege first."""

    GUEST = "guest"
    USER = "user"
    MODERATOR = "moderator"
    ADMIN = "admin"


_RANK: dict[Role, int] = {
    Role.GUEST: 0,
    Role.USER: 1,
    Role.MODERATOR: 2,
    Role.ADMIN: 3,
}


def rank(role: Role) -> int:
    """Return the privilege rank of ``role``."""
    return _RANK[role]


def at_least(role: Role, floor: Role) -> bool:
    """True when ``role`` is at or above ``floor``."""
    return _RANK[role] >= _RANK[floor]


def valid(value: str) -> Role | None:
    """Parse a role name, returning ``None`` for unknown input."""
    try:
        return Role(value)
    except ValueError:
        return None
