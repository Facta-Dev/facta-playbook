"""Forgot, validate, reset, and change password."""

from __future__ import annotations

import hmac
from datetime import timedelta

from app.features.authpassword.domain import PasswordAccount, ResetCode
from app.features.authpassword.ports import PasswordStore
from app.platform.authx import Identity, Passwords
from app.platform.email.outbox import EmailMessage
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource


class PasswordService:
    """Every method is safe against account enumeration except reset itself."""

    def __init__(
        self,
        store: PasswordStore,
        passwords: Passwords,
        ids: IdSource,
        clock: Clock,
        reset_ttl_minutes: int = 30,
        rate_limit: int = 3,
        max_attempts: int = 5,
    ) -> None:
        self._store = store
        self._passwords = passwords
        self._ids = ids
        self._clock = clock
        self._ttl = timedelta(minutes=reset_ttl_minutes)
        self._rate_limit = rate_limit
        self._max_attempts = max_attempts

    async def forgot(self, email: str) -> None:
        """Queue a reset code when allowed; always answer 202."""
        normalized = email.strip().lower()
        account = await self._store.find_account(normalized)
        if account is None:
            return
        now = self._clock.now()
        recent = await self._store.recent_resets(normalized, now - timedelta(minutes=5))
        if recent >= self._rate_limit:
            return
        code = self._ids.numeric_code()
        reset = ResetCode(
            id=self._ids.new_id(),
            email=normalized,
            code=code,
            expires_at=now + self._ttl,
            attempts=0,
        )
        message = EmailMessage(
            to=normalized,
            template_key="reset_password",
            variables={"code": code, "minutes": int(self._ttl.total_seconds() // 60)},
        )
        await self._store.create_reset(reset, message, now)

    async def _check(self, email: str, code: str) -> tuple[ResetCode, PasswordAccount]:
        normalized = email.strip().lower()
        account = await self._store.find_account(normalized)
        reset = await self._store.find_reset(normalized)
        if account is None or reset is None:
            msg = "code is invalid"
            raise E(Code.AUTH_CODE_INVALID, msg, 400)
        if reset.expires_at <= self._clock.now():
            msg = "code expired; request a new one"
            raise E(Code.AUTH_CODE_EXPIRED, msg, 400)
        if reset.attempts >= self._max_attempts:
            msg = "too many attempts; request a new code"
            raise E(Code.AUTH_CODE_ATTEMPTS, msg, 429)
        if not hmac.compare_digest(reset.code, code):
            await self._store.increment_attempts(reset.id)
            msg = "code is invalid"
            raise E(Code.AUTH_CODE_INVALID, msg, 400)
        return reset, account

    async def validate(self, email: str, code: str) -> None:
        """Prove a reset code works without consuming it."""
        await self._check(email, code)

    async def reset(self, email: str, code: str, new_password: str) -> None:
        """Consume the code, set the hash, and revoke every session."""
        reset, account = await self._check(email, code)
        await self._store.consume_reset(reset.email, account.id, self._passwords.hash(new_password))

    async def change(self, identity: Identity, current_password: str, new_password: str) -> None:
        """Require the current password before replacing it."""
        account = await self._store.find_account(identity.email)
        if account is None or not self._passwords.verify(account.password_hash, current_password):
            msg = "current password is incorrect"
            raise E(Code.AUTH_INVALID_CREDENTIALS, msg, 401)
        await self._store.set_password(account.id, self._passwords.hash(new_password))
        await self._store.revoke_sessions(account.id)
