"""Consumer-owned port for the session slice."""

from __future__ import annotations

from datetime import datetime
from typing import Protocol

from app.features.authsession.domain import Account, Session, SessionAnomaly


class SessionStore(Protocol):
    """Storage the session use case needs; no driver types leak through."""

    async def find_account(self, email: str) -> Account | None:
        """Look up by normalized email."""
        ...

    async def get_account(self, user_id: str) -> Account | None:
        """Look up by id."""
        ...

    async def record_login(self, user_id: str, now: datetime) -> None:
        """Update ``last_login``."""
        ...

    async def create_session(self, session: Session) -> None:
        """Persist a new refresh session."""
        ...

    async def find_session(self, token: str) -> Session | None:
        """Look up by refresh token."""
        ...

    async def rotate(
        self, session_id: str, old_token: str, new_token: str, expires_at: datetime, now: datetime
    ) -> bool:
        """Compare-and-swap the refresh token; false means replay lost."""
        ...

    async def deactivate(self, token: str) -> bool:
        """Revoke one session by token."""
        ...

    async def deactivate_all(self, user_id: str) -> int:
        """Revoke every active session for a user."""
        ...

    async def list_sessions(self, user_id: str, now: datetime) -> list[Session]:
        """Active, unexpired sessions for a user."""
        ...

    async def update_profile(
        self, user_id: str, first_name: str, last_name: str, language: str
    ) -> Account | None:
        """Patch the mutable profile fields."""
        ...

    async def anomalies(self, now: datetime) -> list[SessionAnomaly]:
        """Users with active sessions from multiple addresses."""
        ...
