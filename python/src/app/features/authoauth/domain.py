"""Social-login types."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime

from app.shared.rolex import Role

LINK_PURPOSE = "github_link"


@dataclass(frozen=True)
class OAuthUser:
    """The account columns the social flows read."""

    id: str
    email: str
    role: Role
    first_name: str
    last_name: str
    language: str
    is_active: bool
    is_blocked: bool
    is_verified: bool
    google_id: str
    github_id: str


@dataclass(frozen=True)
class OAuthSession:
    """One refresh-token session row created at social login."""

    id: str
    user_id: str
    refresh_token: str
    ip: str
    user_agent: str
    created_at: datetime
    expires_at: datetime


@dataclass(frozen=True)
class OAuthTokens:
    """The pair handed to the client after a social login."""

    access_token: str
    refresh_token: str
