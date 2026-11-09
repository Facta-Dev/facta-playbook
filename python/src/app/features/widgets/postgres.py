"""Postgres adapter for the widget slice."""

from __future__ import annotations

from typing import Any, cast

from app.features.widgets import queries
from app.features.widgets.domain import Widget, WidgetKind
from app.platform.db import Database


def _widget(row: Any) -> Widget:
    return Widget(
        id=str(row["id"]),
        name=str(row["name"]),
        kind=cast(WidgetKind, str(row["kind"])),
        owner_user_id=str(row["owner_user_id"]),
        created_at=row["created_at"],
    )


class PgWidgetStore:
    """Implements ``WidgetStore``; every query filters by owner."""

    def __init__(self, database: Database) -> None:
        self._db = database

    async def insert(self, widget: Widget) -> None:
        async with self._db.session() as session:
            await session.execute(
                queries.INSERT,
                {
                    "id": widget.id,
                    "name": widget.name,
                    "kind": widget.kind,
                    "owner_user_id": widget.owner_user_id,
                    "created_at": widget.created_at,
                },
            )
            await session.commit()

    async def list(self, owner_id: str, limit: int, offset: int) -> list[Widget]:
        async with self._db.session() as session:
            result = await session.execute(
                queries.LIST, {"owner_id": owner_id, "limit": limit, "offset": offset}
            )
            rows = result.mappings().all()
        return [_widget(row) for row in rows]

    async def get(self, widget_id: str, owner_id: str) -> Widget | None:
        async with self._db.session() as session:
            result = await session.execute(
                queries.GET, {"widget_id": widget_id, "owner_id": owner_id}
            )
            row = result.mappings().first()
        return None if row is None else _widget(row)

    async def update(
        self, widget_id: str, owner_id: str, name: str, kind: WidgetKind
    ) -> Widget | None:
        async with self._db.session() as session:
            result = await session.execute(
                queries.UPDATE,
                {"widget_id": widget_id, "owner_id": owner_id, "name": name, "kind": kind},
            )
            row = result.mappings().first()
            await session.commit()
        return None if row is None else _widget(row)

    async def delete(self, widget_id: str, owner_id: str) -> bool:
        async with self._db.session() as session:
            result = await session.execute(
                queries.DELETE, {"widget_id": widget_id, "owner_id": owner_id}
            )
            await session.commit()
        return bool(result.all())
