"""Realtime slice: the single WebSocket endpoint over the platform hub."""

from __future__ import annotations

from fastapi import FastAPI, WebSocket

from app.platform.authx import Tokens
from app.platform.realtime.hub import Hub
from app.platform.realtime.ws import serve


def register(app: FastAPI, hub: Hub, tokens: Tokens) -> None:
    """Mount ``/api/v1/ws``; authentication happens after the upgrade."""

    @app.websocket("/api/v1/ws")
    async def websocket_endpoint(websocket: WebSocket) -> None:
        await serve(websocket, hub, tokens)
