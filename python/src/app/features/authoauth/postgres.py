"""Postgres adapter for the social-login slice."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from app.features.authoauth import queries
from app.features.authoauth.domain import OAuthSession, OAuthUser
from app.platform.db import Database
from app.shared.rolex import Role


def _user(row: Any) -> OAuthUser:
    try:
        role = Role(str(row["role"]).lower())
    except ValueError:
        role = Role.USER
    return OAuthUser(
        id=str(row["id"]),
        email=str(row["email"]),
        role=role,
        first_name=str(row["first_name"] or ""),
        last_name=str(row["last_name"] or ""),
        language=str(row["language"] or "en"),
        is_active=bool(row["is_active"]),
        is_blocked=bool(row["is_blocked"]),
        is_verified=bool(row["is_verified"]),
        google_id=str(row["google_id"] or ""),
        github_id=str(row["github_id"] or ""),
    )


class PgOAuthStore:
    """Implements ``OAuthStore``; every method owns its transaction."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def _one(self, statement: Any, params: dict[str, Any]) -> OAuthUser | None:
        async with self._db.session() as session:
            result = await session.execute(statement, params)
            row = result.mappings().first()
        return None if row is None else _user(row)

    async def find_by_google(self, subject: str) -> OAuthUser | None:
        return await self._one(queries.FIND_BY_GOOGLE, {"subject": subject})

    async def find_by_github(self, subject: str) -> OAuthUser | None:
        return await self._one(queries.FIND_BY_GITHUB, {"subject": subject})

    async def find_by_email(self, email: str) -> OAuthUser | None:
        return await self._one(queries.FIND_BY_EMAIL, {"email": email})

    async def find_by_id(self, user_id: str) -> OAuthUser | None:
        return await self._one(queries.FIND_BY_ID, {"user_id": user_id})

    async def create_user(
        self, user: OAuthUser, password_hash: str, picture: str, now: datetime
    ) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.INSERT_USER,
                {
                    "id": user.id,
                    "email": user.email,
                    "password_hash": password_hash,
                    "role": user.role.value,
                    "first_name": user.first_name,
                    "last_name": user.last_name,
                    "language": user.language,
                    "google_id": user.google_id,
                    "github_id": user.github_id,
                    "picture": picture,
                    "created_at": now,
                },
            )
            await session.commit()

    async def link_google(self, user_id: str, subject: str, picture: str) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.LINK_GOOGLE,
                {"user_id": user_id, "subject": subject, "picture": picture},
            )
            await session.commit()

    async def link_github(self, user_id: str, subject: str, picture: str) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.LINK_GITHUB,
                {"user_id": user_id, "subject": subject, "picture": picture},
            )
            await session.commit()

    async def create_session(self, session_row: OAuthSession) -> None:
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
