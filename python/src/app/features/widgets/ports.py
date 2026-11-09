"""Consumer-owned ports for the widget slice."""

from __future__ import annotations

from typing import Any, Protocol

from app.features.widgets.domain import Widget, WidgetKind


class WidgetStore(Protocol):
    """Persistence, always filtered by owner."""

    async def insert(self, widget: Widget) -> None:
        """Insert one row."""
        ...

    async def list(self, owner_id: str, limit: int, offset: int) -> list[Widget]:
        """One page of the owner's widgets."""
        ...

    async def get(self, widget_id: str, owner_id: str) -> Widget | None:
        """Owner-scoped fetch."""
        ...

    async def update(
        self, widget_id: str, owner_id: str, name: str, kind: WidgetKind
    ) -> Widget | None:
        """Owner-scoped update; returns the fresh row."""
        ...

    async def delete(self, widget_id: str, owner_id: str) -> bool:
        """Owner-scoped delete; ``False`` when nothing matched."""
        ...


class Publisher(Protocol):
    """The realtime surface the use case needs."""

    async def publish(self, room: str, event: dict[str, Any]) -> None:
        """Fan out one event to a room."""
        ...
