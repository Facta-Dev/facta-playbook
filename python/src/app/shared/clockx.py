"""Injectable clock so tests never sleep and never read the wall clock."""

from __future__ import annotations

from datetime import UTC, datetime
from typing import Protocol


class Clock(Protocol):
    """The smallest time surface the application needs."""

    def now(self) -> datetime:
        """Return the current instant, always timezone-aware."""
        ...


class SystemClock:
    """Production clock: UTC, aware."""

    def now(self) -> datetime:
        return datetime.now(UTC)


class FixedClock:
    """Test clock that can be advanced explicitly."""

    def __init__(self, at: datetime) -> None:
        self._at = at if at.tzinfo is not None else at.replace(tzinfo=UTC)

    def now(self) -> datetime:
        return self._at

    def advance(self, seconds: float) -> None:
        from datetime import timedelta

        self._at = self._at + timedelta(seconds=seconds)
