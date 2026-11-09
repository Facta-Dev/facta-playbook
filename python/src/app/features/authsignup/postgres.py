"""Postgres adapter for signup. The transaction boundary lives here."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from app.features.authsignup import queries
from app.features.authsignup.domain import SignupUser, VerificationCode
from app.platform.db import Database
from app.platform.email.outbox import EmailMessage, enqueue_email
from app.shared.apperr import Code, E
from app.shared.rolex import Role


def _user(row: Any) -> SignupUser:
    return SignupUser(
        id=str(row["id"]),
        email=str(row["email"]),
        password_hash=str(row["password_hash"] or ""),
        role=Role(str(row["role"])),
        first_name=str(row["first_name"] or ""),
        last_name=str(row["last_name"] or ""),
        language=str(row["language"] or "en"),
        is_verified=bool(row["is_verified"]),
        created_at=row["created_at"],
    )


def _code(row: Any) -> VerificationCode:
    return VerificationCode(
        id=str(row["id"]),
        user_id=str(row["user_id"]),
        code=str(row["code"]),
        expires_at=row["expires_at"],
        attempts=int(row["attempts"]),
    )


async def _write(session: AsyncSession, user: SignupUser) -> None:
    await session.execute(
        queries.INSERT_USER,
        {
            "id": user.id,
            "email": user.email,
            "password_hash": user.password_hash,
            "role": user.role.value,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "language": user.language,
            "created_at": user.created_at,
        },
    )


async def _write_code(session: AsyncSession, code: VerificationCode, created_at: datetime) -> None:
    await session.execute(queries.DELETE_CODES, {"user_id": code.user_id})
    await session.execute(
        queries.INSERT_CODE,
        {
            "id": code.id,
            "user_id": code.user_id,
            "code": code.code,
            "expires_at": code.expires_at,
            "created_at": created_at,
        },
    )


class PgSignupStore:
    """Implements ``SignupStore``; every write method owns one transaction."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def find_by_email(self, email: str) -> SignupUser | None:
        async with self._db.session() as session:
            result = await session.execute(queries.FIND_USER, {"email": email})
            row = result.mappings().first()
        return None if row is None else _user(row)

    async def register(
        self,
        user: SignupUser,
        code: VerificationCode,
        message: EmailMessage,
        now: datetime,
    ) -> None:
        async with self._db.session() as session:
            try:
                await _write(session, user)
                await _write_code(session, code, now)
                await enqueue_email(session, message, now)
                await session.commit()
            except IntegrityError as exc:
                await session.rollback()
                msg = "email is already registered"
                raise E(Code.AUTH_EMAIL_TAKEN, msg, 409) from exc

    async def replace_code(
        self,
        user_id: str,
        code: VerificationCode,
        message: EmailMessage,
        now: datetime,
    ) -> None:
        async with self._db.session() as session:
            code = VerificationCode(
                id=code.id,
                user_id=user_id,
                code=code.code,
                expires_at=code.expires_at,
                attempts=code.attempts,
            )
            await _write_code(session, code, now)
            await enqueue_email(session, message, now)
            await session.commit()

    async def find_code(self, user_id: str) -> VerificationCode | None:
        async with self._db.session() as session:
            result = await session.execute(queries.FIND_CODE, {"user_id": user_id})
            row = result.mappings().first()
        return None if row is None else _code(row)

    async def increment_attempts(self, code_id: str) -> None:
        async with self._db.session() as session:
            await session.execute(queries.INCREMENT_ATTEMPTS, {"id": code_id})
            await session.commit()

    async def consume_code(self, code_id: str, user_id: str) -> None:
        async with self._db.session() as session:
            await session.execute(queries.DELETE_CODES, {"user_id": user_id})
            await session.execute(queries.MARK_VERIFIED, {"user_id": user_id})
            await session.commit()
