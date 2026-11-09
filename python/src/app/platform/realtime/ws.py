"""WebSocket frame loop with token auth and a 30-second application ping."""

from __future__ import annotations

import asyncio
import contextlib
import logging
from typing import Any

from fastapi import WebSocket, WebSocketDisconnect

from app.platform.authx import Tokens
from app.platform.realtime.hub import Hub
from app.shared.apperr import AppError

_log = logging.getLogger("app.realtime")
_PING_SECONDS = 30.0


def _frame(kind: str, payload: dict[str, Any] | None = None) -> dict[str, Any]:
    return {"type": kind, **(payload or {})}


async def _ping(websocket: WebSocket, stop: asyncio.Event) -> None:
    while not stop.is_set():
        try:
            await asyncio.wait_for(stop.wait(), timeout=_PING_SECONDS)
        except TimeoutError:
            await websocket.send_json(_frame("ping"))


async def serve(websocket: WebSocket, hub: Hub, tokens: Tokens) -> None:
    """Authenticate after upgrade, then relay room events until disconnect."""
    token = websocket.query_params.get("token", "")
    identity = None
    if token:
        try:
            identity = tokens.verify_access(token)
        except AppError:
            identity = None
    if identity is None:
        # Complete the handshake first: the browser then sees the 4001 close
        # code, not an opaque HTTP 403 rejection.
        await websocket.accept()
        await websocket.close(code=4001, reason="authentication_failed")
        return

    subscription = await hub.subscribe(f"user:{identity.user_id}")
    await websocket.accept()
    await websocket.send_json(_frame("hello", {"user_id": identity.user_id}))
    stop = asyncio.Event()
    ping_task = asyncio.create_task(_ping(websocket, stop))
    try:
        while True:
            outgoing = asyncio.create_task(subscription.queue.get())
            incoming = asyncio.create_task(websocket.receive_text())
            done, pending = await asyncio.wait(
                {outgoing, incoming}, return_when=asyncio.FIRST_COMPLETED
            )
            for task in pending:
                task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await task
            if incoming in done:
                message = incoming.result()
                if message == "ping":
                    await websocket.send_json(_frame("pong"))
                continue
            await websocket.send_json(outgoing.result())
    except WebSocketDisconnect:
        pass
    finally:
        stop.set()
        ping_task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await ping_task
        await hub.unsubscribe(subscription)
