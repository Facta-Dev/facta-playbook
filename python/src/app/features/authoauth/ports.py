"""Consumer-owned ports for the social flows."""

from __future__ import annotations

from datetime import datetime
from typing import Protocol

from app.features.authoauth.domain import OAuthSession, OAuthUser
from app.platform.oauthx import ExternalIdentity


class GoogleVerifier(Protocol):
    """Verifies a Google One-Tap credential."""

    @property
    def configured(self) -> bool:
        """Whether a client id is present."""
        ...

    async def verify(self, id_token: str) -> ExternalIdentity:
        """Return the verified provider identity."""
        ...


class GithubClient(Protocol):
    """Builds the authorize URL and exchanges codes."""

    @property
    def configured(self) -> bool:
        """Whether client id and secret are present."""
        ...

    def authorize_url(self, redirect_uri: str, state: str) -> str:
        """GitHub's consent URL."""
        ...

    async def exchange(self, code: str) -> ExternalIdentity:
        """Trade a code for the verified profile."""
        ...


class OAuthStore(Protocol):
    """Persistence for users and sessions in the social flows."""

    async def find_by_google(self, subject: str) -> OAuthUser | None:
        """Account already linked to a Google subject."""
        ...

    async def find_by_github(self, subject: str) -> OAuthUser | None:
        """Account already linked to a GitHub subject."""
        ...

    async def find_by_email(self, email: str) -> OAuthUser | None:
        """Account by email."""
        ...

    async def find_by_id(self, user_id: str) -> OAuthUser | None:
        """Account by id."""
        ...

    async def create_user(
        self,
        user: OAuthUser,
        password_hash: str,
        picture: str,
        now: datetime,
    ) -> None:
        """Provision a social account (already verified)."""
        ...

    async def link_google(self, user_id: str, subject: str, picture: str) -> None:
        """Attach a Google subject and verify the email."""
        ...

    async def link_github(self, user_id: str, subject: str, picture: str) -> None:
        """Attach a GitHub subject and verify the email."""
        ...

    async def create_session(self, session: OAuthSession) -> None:
        """Persist a refresh session."""
        ...
