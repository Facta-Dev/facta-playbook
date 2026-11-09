"""Postgres adapter for admin user management."""

from __future__ import annotations

from typing import Any
from uuid import uuid4

from app.features.adminusers import queries
from app.features.adminusers.domain import AdminUser, Patch
from app.platform.db import Database
from app.shared.rolex import Role


def _admin(row: Any) -> AdminUser:
    try:
        role = Role(str(row["role"]).lower())
    except ValueError:
        role = Role.USER
    return AdminUser(
        id=str(row["id"]),
        email=str(row["email"]),
        first_name=str(row["first_name"] or ""),
        last_name=str(row["last_name"] or ""),
        role=role,
        is_active=bool(row["is_active"]),
        is_blocked=bool(row["is_blocked"]),
        is_verified=bool(row["is_verified"]),
        language=str(row["language"] or "en"),
        created_at=row["created_at"],
        last_login=row["last_login"],
    )


class PgAdminUserStore:
    """Implements ``AdminUserStore``; every method owns its transaction."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def list(self, limit: int, offset: int) -> list[AdminUser]:
        async with self._db.session() as session:
            result = await session.execute(queries.LIST, {"limit": limit, "offset": offset})
            rows = result.mappings().all()
        return [_admin(row) for row in rows]

    async def count(self) -> int:
        async with self._db.session() as session:
            result = await session.execute(queries.COUNT)
            row = result.mappings().first()
        return int(row["n"]) if row is not None else 0

    async def get(self, user_id: str) -> AdminUser | None:
        async with self._db.session() as session:
            result = await session.execute(queries.GET, {"user_id": user_id})
            row = result.mappings().first()
        return None if row is None else _admin(row)

    async def get_by_email(self, email: str) -> AdminUser | None:
        async with self._db.session() as session:
            result = await session.execute(queries.GET_BY_EMAIL, {"email": email})
            row = result.mappings().first()
        return None if row is None else _admin(row)

    async def update(self, user_id: str, patch: Patch) -> AdminUser | None:
        async with self._db.session() as session:
            result = await session.execute(
                queries.UPDATE,
                {
                    "user_id": user_id,
                    "is_active": patch.is_active,
                    "is_blocked": patch.is_blocked,
                    "role": patch.role.value if patch.role is not None else None,
                },
            )
            row = result.mappings().first()
            await session.commit()
        return None if row is None else _admin(row)

    async def upgrade_to_admin(self, user_id: str) -> None:
        async with self._db.session() as session:
            await session.execute(queries.UPGRADE_TO_ADMIN, {"user_id": user_id})
            await session.commit()

    async def write_audit(
        self,
        actor_id: str,
        action: str,
        entity: str,
        entity_id: str,
        payload: str,
        ip: str,
        user_agent: str,
    ) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.INSERT_AUDIT,
                {
                    "id": str(uuid4()),
                    "actor_id": actor_id or None,
                    "action": action,
                    "entity": entity,
                    "entity_id": entity_id,
                    "payload": payload,
                    "ip": ip,
                    "user_agent": user_agent,
                },
            )
            await session.commit()
