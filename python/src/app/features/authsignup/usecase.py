"""Signup use case: create the account, verify the email, resend codes."""

from __future__ import annotations

import hmac
from datetime import timedelta

from app.features.authsignup.domain import SignupResult, SignupUser, VerificationCode
from app.features.authsignup.ports import SignupStore
from app.platform.authx import Passwords
from app.platform.email.outbox import EmailMessage
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource
from app.shared.rolex import Role


class SignupService:
    """One use case per public operation; the store owns atomicity."""

    def __init__(
        self,
        store: SignupStore,
        passwords: Passwords,
        ids: IdSource,
        clock: Clock,
        code_ttl_hours: int = 24,
        max_attempts: int = 5,
    ) -> None:
        self._store = store
        self._passwords = passwords
        self._ids = ids
        self._clock = clock
        self._ttl = timedelta(hours=code_ttl_hours)
        self._max_attempts = max_attempts

    def _issue(self, user_id: str, email: str) -> tuple[VerificationCode, EmailMessage]:
        now = self._clock.now()
        code = self._ids.numeric_code()
        verification = VerificationCode(
            id=self._ids.new_id(),
            user_id=user_id,
            code=code,
            expires_at=now + self._ttl,
            attempts=0,
        )
        message = EmailMessage(
            to=email,
            template_key="verify_email",
            variables={"code": code, "hours": self._ttl.total_seconds() / 3600},
        )
        return verification, message

    async def signup(
        self,
        email: str,
        password: str,
        first_name: str = "",
        last_name: str = "",
        language: str = "en",
    ) -> SignupResult:
        """Create an unverified account or re-issue its code."""
        normalized = email.strip().lower()
        existing = await self._store.find_by_email(normalized)
        if existing is not None:
            if existing.is_verified:
                msg = "email is already registered"
                raise E(Code.AUTH_EMAIL_TAKEN, msg, 409)
            verification, message = self._issue(existing.id, existing.email)
            await self._store.replace_code(existing.id, verification, message, self._clock.now())
            return SignupResult(user_id=existing.id, code_queued=True)

        now = self._clock.now()
        user = SignupUser(
            id=self._ids.new_id(),
            email=normalized,
            password_hash=self._passwords.hash(password),
            role=Role.USER,
            first_name=first_name,
            last_name=last_name,
            language=language,
            is_verified=False,
            created_at=now,
        )
        verification, message = self._issue(user.id, user.email)
        await self._store.register(user, verification, message, now)
        return SignupResult(user_id=user.id, code_queued=True)

    async def verify(self, email: str, code: str) -> str:
        """Consume a valid code and mark the account verified."""
        user = await self._store.find_by_email(email.strip().lower())
        if user is None:
            msg = "code is invalid"
            raise E(Code.AUTH_CODE_INVALID, msg, 400)
        verification = await self._store.find_code(user.id)
        if verification is None:
            msg = "no active code; request a new one"
            raise E(Code.AUTH_CODE_INVALID, msg, 400)
        now = self._clock.now()
        if verification.expires_at <= now:
            msg = "code expired; request a new one"
            raise E(Code.AUTH_CODE_EXPIRED, msg, 400)
        if verification.attempts >= self._max_attempts:
            msg = "too many attempts; request a new code"
            raise E(Code.AUTH_CODE_ATTEMPTS, msg, 429)
        if not hmac.compare_digest(verification.code, code):
            await self._store.increment_attempts(verification.id)
            msg = "code is invalid"
            raise E(Code.AUTH_CODE_INVALID, msg, 400)
        await self._store.consume_code(verification.id, user.id)
        return user.id

    async def resend(self, email: str) -> None:
        """Re-issue a code without revealing whether the account exists."""
        user = await self._store.find_by_email(email.strip().lower())
        if user is None:
            return
        if user.is_verified:
            msg = "email is already verified"
            raise E(Code.AUTH_EMAIL_TAKEN, msg, 409)
        verification, message = self._issue(user.id, user.email)
        await self._store.replace_code(user.id, verification, message, self._clock.now())
