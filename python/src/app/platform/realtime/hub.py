"""In-process pub/sub. One room per user; queues are bounded and drop-oldest."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Subscription:
    """A subscriber's queue and the room it belongs to."""

    room: str
    queue: asyncio.Queue[dict[str, Any]] = field(default_factory=lambda: asyncio.Queue(32))


class Hub:
    """Rooms are process-local; a second replica needs a real broker."""

    def __init__(self) -> None:
        self._rooms: dict[str, set[asyncio.Queue[dict[str, Any]]]] = {}
        self._lock = asyncio.Lock()

    async def subscribe(self, room: str) -> Subscription:
        subscription = Subscription(room=room)
        async with self._lock:
            self._rooms.setdefault(room, set()).add(subscription.queue)
        return subscription

    async def unsubscribe(self, subscription: Subscription) -> None:
        async with self._lock:
            listeners = self._rooms.get(subscription.room)
            if listeners is None:
                return
            listeners.discard(subscription.queue)
            if not listeners:
                self._rooms.pop(subscription.room, None)

    async def publish(self, room: str, event: dict[str, Any]) -> None:
        async with self._lock:
            listeners = list(self._rooms.get(room, ()))
        for queue in listeners:
            try:
                queue.put_nowait(event)
            except asyncio.QueueFull:
                # ponytail: drop the oldest event for a stalled subscriber;
                # a broker with per-consumer retention if delivery matters.
                _ = queue.get_nowait()
                queue.put_nowait(event)
