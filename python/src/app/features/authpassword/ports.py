"""Consumer-owned port for password reset and change."""

from __future__ import annotations

from datetime import datetime
from typing import Protocol

from app.features.authpassword.domain import PasswordAccount, ResetCode
from app.platform.email.outbox import EmailMessage


class PasswordStore(Protocol):
    """Storage the password use cases need."""

    async def find_account(self, email: str) -> PasswordAccount | None:
        """Look up by normalized email."""
        ...

    async def recent_resets(self, email: str, since: datetime) -> int:
        """Count resets created since ``since`` for rate limiting."""
        ...

    async def create_reset(self, reset: ResetCode, message: EmailMessage, now: datetime) -> None:
        """Insert the code and its outbox job in one transaction."""
        ...

    async def find_reset(self, email: str) -> ResetCode | None:
        """Newest reset row for an email."""
        ...

    async def increment_attempts(self, reset_id: str) -> None:
        """Record one wrong guess."""
        ...

    async def consume_reset(self, email: str, user_id: str, password_hash: str) -> None:
        """Delete the code, set the hash, revoke sessions — one transaction."""
        ...

    async def set_password(self, user_id: str, password_hash: str) -> None:
        """Set a new hash for a change-password call."""
        ...

    async def revoke_sessions(self, user_id: str) -> int:
        """Deactivate every session for a user."""
        ...
