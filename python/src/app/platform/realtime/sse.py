"""Server-sent events framing and a cancellable stream."""

from __future__ import annotations

import asyncio
import json
from collections.abc import AsyncIterator
from typing import Any


def format_event(event: dict[str, Any]) -> str:
    """One SSE frame; JSON on a single data line."""
    return f"data: {json.dumps(event, separators=(',', ':'))}\n\n"


async def stream(
    queue: asyncio.Queue[dict[str, Any]],
    heartbeat_seconds: float = 15.0,
) -> AsyncIterator[str]:
    """Yield queued events, or a comment heartbeat to keep proxies open."""
    while True:
        try:
            event = await asyncio.wait_for(queue.get(), timeout=heartbeat_seconds)
        except TimeoutError:
            yield ": keep-alive\n\n"
        else:
            yield format_event(event)
