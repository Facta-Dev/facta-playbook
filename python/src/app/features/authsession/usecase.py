"""Login gates, refresh rotation, and session management."""

from __future__ import annotations

from datetime import timedelta

from app.features.authsession.domain import (
    Account,
    Session,
    SessionAnomaly,
    SessionView,
    TokenPair,
)
from app.features.authsession.ports import SessionStore
from app.platform.authx import Identity, Passwords, Tokens
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource

# A fixed hash keeps the unknown-email branch as slow as the wrong-password
# branch, so login timing does not disclose which accounts exist.
_DUMMY_HASH = (
    "$argon2id$v=19$m=65536,t=3,p=4$c2FsdHNhbHRzYWx0c2FsdA$"
    "9pK0rQd7fS5m8n1o2p3q4r5s6t7u8v9w0x1y2z3a4b5"
)


class SessionService:
    """One method per public operation; gates always run in the same order."""

    def __init__(
        self,
        store: SessionStore,
        passwords: Passwords,
        tokens: Tokens,
        ids: IdSource,
        clock: Clock,
        refresh_ttl_hours: int = 720,
    ) -> None:
        self._store = store
        self._passwords = passwords
        self._tokens = tokens
        self._ids = ids
        self._clock = clock
        self._refresh_ttl = timedelta(hours=refresh_ttl_hours)

    def _issue(self, account: Account) -> TokenPair:
        identity = Identity(user_id=account.id, email=account.email, role=account.role)
        return TokenPair(
            access_token=self._tokens.mint_access(identity),
            refresh_token=self._ids.opaque_token(),
        )

    async def login(self, email: str, password: str, ip: str, user_agent: str) -> TokenPair:
        """Password first, then blocked -> inactive -> unverified."""
        account = await self._store.find_account(email.strip().lower())
        stored = account.password_hash if account is not None else _DUMMY_HASH
        if not self._passwords.verify(stored, password) or account is None:
            msg = "invalid email or password"
            raise E(Code.AUTH_INVALID_CREDENTIALS, msg, 401)
        if account.is_blocked:
            msg = "account is blocked"
            raise E(Code.AUTH_ACCOUNT_BLOCKED, msg, 403)
        if not account.is_active:
            msg = "account is inactive"
            raise E(Code.AUTH_ACCOUNT_INACTIVE, msg, 403)
        if not account.is_verified:
            msg = "email is not verified"
            raise E(Code.AUTH_EMAIL_UNVERIFIED, msg, 403)

        now = self._clock.now()
        pair = self._issue(account)
        await self._store.create_session(
            Session(
                id=self._ids.new_id(),
                user_id=account.id,
                refresh_token=pair.refresh_token,
                ip=ip,
                user_agent=user_agent,
                is_active=True,
                created_at=now,
                expires_at=now + self._refresh_ttl,
            )
        )
        await self._store.record_login(account.id, now)
        return pair

    async def refresh(self, refresh_token: str) -> TokenPair:
        """Rotate in place; a replayed token loses the compare-and-swap."""
        session = await self._store.find_session(refresh_token)
        now = self._clock.now()
        if session is None or not session.is_active or session.expires_at <= now:
            msg = "invalid or expired refresh token"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 401)
        account = await self._store.get_account(session.user_id)
        if account is None or account.is_blocked or not account.is_active:
            msg = "session is no longer usable"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 401)
        next_token = self._ids.opaque_token()
        rotated = await self._store.rotate(
            session.id, refresh_token, next_token, now + self._refresh_ttl, now
        )
        if not rotated:
            msg = "refresh token was already used"
            raise E(Code.AUTH_INVALID_TOKEN, msg, 401)
        identity = Identity(user_id=account.id, email=account.email, role=account.role)
        return TokenPair(access_token=self._tokens.mint_access(identity), refresh_token=next_token)

    async def logout(self, refresh_token: str) -> None:
        """Revoke the presented session; unknown tokens are a no-op."""
        await self._store.deactivate(refresh_token)

    async def me(self, identity: Identity) -> Account:
        """Return the authenticated account."""
        account = await self._store.get_account(identity.user_id)
        if account is None:
            msg = "account not found"
            raise E(Code.NOT_FOUND, msg, 404)
        return account

    async def update_me(
        self,
        identity: Identity,
        first_name: str | None,
        last_name: str | None,
        language: str | None,
    ) -> Account:
        """Patch only the caller's own profile fields."""
        current = await self.me(identity)
        account = await self._store.update_profile(
            current.id,
            first_name if first_name is not None else current.first_name,
            last_name if last_name is not None else current.last_name,
            language if language is not None else current.language,
        )
        if account is None:
            msg = "account not found"
            raise E(Code.NOT_FOUND, msg, 404)
        return account

    async def sessions(self, identity: Identity, current_token: str = "") -> list[SessionView]:
        """List the caller's active sessions."""
        rows = await self._store.list_sessions(identity.user_id, self._clock.now())
        return [
            SessionView(
                id=row.id,
                ip=row.ip,
                user_agent=row.user_agent,
                created_at=row.created_at,
                expires_at=row.expires_at,
                current=row.refresh_token == current_token,
            )
            for row in rows
        ]

    async def revoke_current(self, refresh_token: str) -> bool:
        """Revoke the presented session."""
        return await self._store.deactivate(refresh_token)

    async def revoke_all(self, identity: Identity) -> int:
        """Revoke every session for the caller."""
        return await self._store.deactivate_all(identity.user_id)

    async def admin_sessions(self, user_id: str) -> list[SessionView]:
        """Admin view of another user's sessions."""
        rows = await self._store.list_sessions(user_id, self._clock.now())
        return [
            SessionView(
                id=row.id,
                ip=row.ip,
                user_agent=row.user_agent,
                created_at=row.created_at,
                expires_at=row.expires_at,
                current=False,
            )
            for row in rows
        ]

    async def admin_revoke_all(self, user_id: str) -> int:
        """Admin revoke of every session for a user."""
        return await self._store.deactivate_all(user_id)

    async def anomalies(self) -> list[SessionAnomaly]:
        """Users whose active sessions span more than one address."""
        return await self._store.anomalies(self._clock.now())
