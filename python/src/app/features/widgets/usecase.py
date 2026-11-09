"""Owner-scoped widget CRUD; lifecycle changes are published."""

from __future__ import annotations

from typing import Any, cast

from app.features.widgets.domain import WIDGET_KINDS, Widget, WidgetKind
from app.features.widgets.ports import Publisher, WidgetStore
from app.shared.apperr import Code, E
from app.shared.clockx import Clock
from app.shared.idgen import IdSource


def _room(owner_id: str) -> str:
    return f"user:{owner_id}"


class WidgetService:
    """Reads and writes are filtered by the authenticated owner."""

    def __init__(
        self,
        store: WidgetStore,
        publisher: Publisher | None,
        ids: IdSource,
        clock: Clock,
    ) -> None:
        self._store = store
        self._publisher = publisher
        self._ids = ids
        self._clock = clock

    def _kind(self, kind: str) -> WidgetKind:
        if kind not in WIDGET_KINDS:
            msg = "unknown kind"
            raise E(Code.VALIDATION_FAILED, msg, 422)
        return cast(WidgetKind, kind)

    async def _publish(self, owner_id: str, event: dict[str, Any]) -> None:
        if self._publisher is not None:
            await self._publisher.publish(_room(owner_id), event)

    async def list(self, owner_id: str, limit: int, offset: int) -> list[Widget]:
        """One page of the owner's widgets."""
        return await self._store.list(owner_id, limit, offset)

    async def create(self, owner_id: str, name: str, kind: str) -> Widget:
        """Insert and announce."""
        widget = Widget(
            id=self._ids.new_id(),
            name=name,
            kind=self._kind(kind),
            owner_user_id=owner_id,
            created_at=self._clock.now(),
        )
        await self._store.insert(widget)
        await self._publish(owner_id, {"type": "widget.created", "widget": name})
        return widget

    async def get(self, widget_id: str, owner_id: str) -> Widget:
        """Owner-scoped fetch or 404."""
        widget = await self._store.get(widget_id, owner_id)
        if widget is None:
            msg = "widget not found"
            raise E(Code.WIDGET_NOT_FOUND, msg, 404)
        return widget

    async def update(self, widget_id: str, owner_id: str, name: str, kind: str) -> Widget:
        """Owner-scoped update and announce."""
        widget = await self._store.update(widget_id, owner_id, name, self._kind(kind))
        if widget is None:
            msg = "widget not found"
            raise E(Code.WIDGET_NOT_FOUND, msg, 404)
        await self._publish(owner_id, {"type": "widget.updated", "widget": name})
        return widget

    async def delete(self, widget_id: str, owner_id: str) -> None:
        """Owner-scoped delete and announce."""
        if not await self._store.delete(widget_id, owner_id):
            msg = "widget not found"
            raise E(Code.WIDGET_NOT_FOUND, msg, 404)
        await self._publish(owner_id, {"type": "widget.deleted", "widget": widget_id})
