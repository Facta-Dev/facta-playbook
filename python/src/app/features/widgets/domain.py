"""Widget types — the example owner-scoped CRUD domain."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from typing import Literal

WidgetKind = Literal["sprocket", "cog", "flange"]
WIDGET_KINDS: tuple[str, ...] = ("sprocket", "cog", "flange")


@dataclass(frozen=True)
class Widget:
    """One owned row."""

    id: str
    name: str
    kind: WidgetKind
    owner_user_id: str
    created_at: datetime
