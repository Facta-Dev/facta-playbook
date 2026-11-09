"""Postgres adapter for sessions and account reads."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from sqlalchemy.ext.asyncio import AsyncSession

from app.features.authsession import queries
from app.features.authsession.domain import Account, Session, SessionAnomaly
from app.platform.db import Database
from app.shared.rolex import Role


def _account(row: Any) -> Account:
    return Account(
        id=str(row["id"]),
        email=str(row["email"]),
        password_hash=str(row["password_hash"] or ""),
        role=Role(str(row["role"])),
        first_name=str(row["first_name"] or ""),
        last_name=str(row["last_name"] or ""),
        language=str(row["language"] or "en"),
        is_active=bool(row["is_active"]),
        is_blocked=bool(row["is_blocked"]),
        is_verified=bool(row["is_verified"]),
        last_login=row["last_login"],
    )


def _session(row: Any) -> Session:
    return Session(
        id=str(row["id"]),
        user_id=str(row["user_id"]),
        refresh_token=str(row["refresh_token"]),
        ip=str(row["ip"] or ""),
        user_agent=str(row["user_agent"] or ""),
        is_active=bool(row["is_active"]),
        created_at=row["created_at"],
        expires_at=row["expires_at"],
    )


async def _one(session: AsyncSession, statement: Any, params: dict[str, object]) -> Any:
    result = await session.execute(statement, params)
    return result.mappings().first()


class PgSessionStore:
    """Implements ``SessionStore`` over the pool."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def find_account(self, email: str) -> Account | None:
        async with self._db.session() as session:
            row = await _one(session, queries.FIND_ACCOUNT_BY_EMAIL, {"email": email})
        return None if row is None else _account(row)

    async def get_account(self, user_id: str) -> Account | None:
        async with self._db.session() as session:
            row = await _one(session, queries.FIND_ACCOUNT_BY_ID, {"user_id": user_id})
        return None if row is None else _account(row)

    async def record_login(self, user_id: str, now: datetime) -> None:
        async with self._db.session() as session:
            await session.execute(queries.RECORD_LOGIN, {"user_id": user_id, "now": now})
            await session.commit()

    async def create_session(self, session_row: Session) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.INSERT_SESSION,
                {
                    "id": session_row.id,
                    "user_id": session_row.user_id,
                    "refresh_token": session_row.refresh_token,
                    "ip": session_row.ip,
                    "user_agent": session_row.user_agent,
                    "created_at": session_row.created_at,
                    "expires_at": session_row.expires_at,
                },
            )
            await session.commit()

    async def find_session(self, token: str) -> Session | None:
        async with self._db.session() as session:
            row = await _one(session, queries.FIND_SESSION, {"token": token})
        return None if row is None else _session(row)

    async def rotate(
        self,
        session_id: str,
        old_token: str,
        new_token: str,
        expires_at: datetime,
        now: datetime,
    ) -> bool:
        async with self._db.session() as session:
            result = await session.execute(
                queries.ROTATE,
                {
                    "id": session_id,
                    "old_token": old_token,
                    "new_token": new_token,
                    "expires_at": expires_at,
                    "now": now,
                },
            )
            await session.commit()
        return bool(result.all())

    async def deactivate(self, token: str) -> bool:
        async with self._db.session() as session:
            result = await session.execute(queries.DEACTIVATE, {"token": token})
            await session.commit()
        return bool(result.all())

    async def deactivate_all(self, user_id: str) -> int:
        async with self._db.session() as session:
            result = await session.execute(queries.DEACTIVATE_ALL, {"user_id": user_id})
            await session.commit()
        return len(result.all())

    async def list_sessions(self, user_id: str, now: datetime) -> list[Session]:
        async with self._db.session() as session:
            result = await session.execute(queries.LIST_SESSIONS, {"user_id": user_id, "now": now})
            rows = result.mappings().all()
        return [_session(row) for row in rows]

    async def update_profile(
        self, user_id: str, first_name: str, last_name: str, language: str
    ) -> Account | None:
        async with self._db.session() as session:
            row = await _one(
                session,
                queries.UPDATE_PROFILE,
                {
                    "user_id": user_id,
                    "first_name": first_name,
                    "last_name": last_name,
                    "language": language,
                },
            )
            await session.commit()
        return None if row is None else _account(row)

    async def anomalies(self, now: datetime) -> list[SessionAnomaly]:
        async with self._db.session() as session:
            result = await session.execute(queries.ANOMALIES, {"now": now})
            rows = result.mappings().all()
        return [
            SessionAnomaly(
                user_id=str(row["user_id"]),
                email=str(row["email"]),
                distinct_ips=int(row["distinct_ips"]),
                sessions=int(row["sessions"]),
            )
            for row in rows
        ]
