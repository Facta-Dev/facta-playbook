"""Consumer-owned port for signup persistence.

``register`` and ``replace_code`` are unit-of-work methods: each commits the
state write and the outbox row in ONE transaction. The port is the atomicity
boundary; callers never see a session.
"""

from __future__ import annotations

from datetime import datetime
from typing import Protocol

from app.features.authsignup.domain import SignupUser, VerificationCode
from app.platform.email.outbox import EmailMessage


class SignupStore(Protocol):
    """Everything the signup use case needs from storage."""

    async def find_by_email(self, email: str) -> SignupUser | None:
        """Look up a user by normalized email."""
        ...

    async def register(
        self,
        user: SignupUser,
        code: VerificationCode,
        message: EmailMessage,
        now: datetime,
    ) -> None:
        """Insert user + code + outbox job atomically."""
        ...

    async def replace_code(
        self,
        user_id: str,
        code: VerificationCode,
        message: EmailMessage,
        now: datetime,
    ) -> None:
        """Replace the active code and enqueue its email atomically."""
        ...

    async def find_code(self, user_id: str) -> VerificationCode | None:
        """Return the newest active verification row."""
        ...

    async def increment_attempts(self, code_id: str) -> None:
        """Record one wrong guess."""
        ...

    async def consume_code(self, code_id: str, user_id: str) -> None:
        """Delete the code and mark the user verified atomically."""
        ...
