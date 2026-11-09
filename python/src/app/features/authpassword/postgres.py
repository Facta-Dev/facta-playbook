"""Postgres adapter for password reset and change."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from app.features.authpassword import queries
from app.features.authpassword.domain import PasswordAccount, ResetCode
from app.platform.db import Database
from app.platform.email.outbox import EmailMessage, enqueue_email


def _account(row: Any) -> PasswordAccount:
    return PasswordAccount(
        id=str(row["id"]),
        email=str(row["email"]),
        password_hash=str(row["password_hash"] or ""),
    )


def _reset(row: Any) -> ResetCode:
    return ResetCode(
        id=str(row["id"]),
        email=str(row["email"]),
        code=str(row["code"]),
        expires_at=row["expires_at"],
        attempts=int(row["attempts"]),
    )


class PgPasswordStore:
    """Implements ``PasswordStore``; multi-write methods own one transaction."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def find_account(self, email: str) -> PasswordAccount | None:
        async with self._db.session() as session:
            result = await session.execute(queries.FIND_ACCOUNT, {"email": email})
            row = result.mappings().first()
        return None if row is None else _account(row)

    async def recent_resets(self, email: str, since: datetime) -> int:
        async with self._db.session() as session:
            result = await session.execute(queries.RECENT_RESETS, {"email": email, "since": since})
            row = result.mappings().first()
        return int(row["n"]) if row is not None else 0

    async def create_reset(self, reset: ResetCode, message: EmailMessage, now: datetime) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.INSERT_RESET,
                {
                    "id": reset.id,
                    "email": reset.email,
                    "code": reset.code,
                    "expires_at": reset.expires_at,
                    "created_at": now,
                },
            )
            await enqueue_email(session, message, now)
            await session.commit()

    async def find_reset(self, email: str) -> ResetCode | None:
        async with self._db.session() as session:
            result = await session.execute(queries.FIND_RESET, {"email": email})
            row = result.mappings().first()
        return None if row is None else _reset(row)

    async def increment_attempts(self, reset_id: str) -> None:
        async with self._db.session() as session:
            await session.execute(queries.INCREMENT_ATTEMPTS, {"id": reset_id})
            await session.commit()

    async def consume_reset(self, email: str, user_id: str, password_hash: str) -> None:
        async with self._db.session() as session:
            await session.execute(queries.DELETE_RESET, {"email": email})
            await session.execute(
                queries.SET_PASSWORD, {"user_id": user_id, "password_hash": password_hash}
            )
            await session.execute(queries.REVOKE_SESSIONS, {"user_id": user_id})
            await session.commit()

    async def set_password(self, user_id: str, password_hash: str) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.SET_PASSWORD, {"user_id": user_id, "password_hash": password_hash}
            )
            await session.commit()

    async def revoke_sessions(self, user_id: str) -> int:
        async with self._db.session() as session:
            result = await session.execute(queries.REVOKE_SESSIONS, {"user_id": user_id})
            await session.commit()
        return len(result.all())
