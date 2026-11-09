"""Session and account types for login, refresh, and session management."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime

from app.shared.rolex import Role


@dataclass(frozen=True)
class Account:
    """The identity fields the session slice reads."""

    id: str
    email: str
    password_hash: str
    role: Role
    first_name: str
    last_name: str
    language: str
    is_active: bool
    is_blocked: bool
    is_verified: bool
    last_login: datetime | None


@dataclass(frozen=True)
class Session:
    """One refresh-token session row."""

    id: str
    user_id: str
    refresh_token: str
    ip: str
    user_agent: str
    is_active: bool
    created_at: datetime
    expires_at: datetime


@dataclass(frozen=True)
class SessionView:
    """A session as the client sees it (token included for current only)."""

    id: str
    ip: str
    user_agent: str
    created_at: datetime
    expires_at: datetime
    current: bool


@dataclass(frozen=True)
class TokenPair:
    """Access + refresh issued by login and refresh."""

    access_token: str
    refresh_token: str


@dataclass(frozen=True)
class SessionAnomaly:
    """A user with active sessions from more than one address."""

    user_id: str
    email: str
    distinct_ips: int
    sessions: int
