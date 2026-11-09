"""Widget CRUD and the owner's SSE event stream."""

from __future__ import annotations

from collections.abc import AsyncIterator
from datetime import datetime
from typing import Annotated

from fastapi import APIRouter, Query, Request, status
from fastapi.responses import StreamingResponse
from pydantic import BaseModel, StringConstraints

from app.features.widgets.domain import Widget
from app.features.widgets.usecase import WidgetService
from app.platform.authx import IdentityDep
from app.platform.envelope import pagination
from app.platform.realtime.hub import Hub
from app.platform.realtime.sse import stream


class WidgetRequest(BaseModel):
    name: Annotated[str, StringConstraints(min_length=1, max_length=120)]
    kind: Annotated[str, StringConstraints(min_length=1, max_length=32)]


class WidgetResponse(BaseModel):
    id: str
    name: str
    kind: str
    owner_user_id: str
    created_at: datetime


class WidgetListResponse(BaseModel):
    items: list[WidgetResponse]


def _model(widget: Widget) -> WidgetResponse:
    return WidgetResponse(
        id=widget.id,
        name=widget.name,
        kind=widget.kind,
        owner_user_id=widget.owner_user_id,
        created_at=widget.created_at,
    )


def build_router(service: WidgetService) -> APIRouter:
    """Every route is authenticated and owner-scoped."""
    router = APIRouter(tags=["widgets"])

    @router.get("/widgets")
    async def list_widgets(
        identity: IdentityDep,
        page: Annotated[int, Query(ge=1)] = 1,
        page_size: Annotated[int, Query(ge=1, le=100)] = 20,
    ) -> WidgetListResponse:
        safe_page, safe_size = pagination(page, page_size)
        rows = await service.list(identity.user_id, safe_size, (safe_page - 1) * safe_size)
        return WidgetListResponse(items=[_model(row) for row in rows])

    @router.post("/widgets", status_code=status.HTTP_201_CREATED)
    async def create_widget(body: WidgetRequest, identity: IdentityDep) -> WidgetResponse:
        return _model(await service.create(identity.user_id, body.name, body.kind))

    @router.get("/widgets/events")
    async def widget_events(request: Request, identity: IdentityDep) -> StreamingResponse:
        hub: Hub = request.app.state.hub
        subscription = await hub.subscribe(f"user:{identity.user_id}")

        async def body() -> AsyncIterator[str]:
            try:
                async for frame in stream(subscription.queue):
                    if await request.is_disconnected():
                        return
                    yield frame
            finally:
                await hub.unsubscribe(subscription)

        return StreamingResponse(
            body(),
            media_type="text/event-stream",
            headers={"cache-control": "no-cache", "x-accel-buffering": "no"},
        )

    @router.get("/widgets/{widget_id}")
    async def get_widget(widget_id: str, identity: IdentityDep) -> WidgetResponse:
        return _model(await service.get(widget_id, identity.user_id))

    @router.put("/widgets/{widget_id}")
    async def update_widget(
        widget_id: str, body: WidgetRequest, identity: IdentityDep
    ) -> WidgetResponse:
        return _model(await service.update(widget_id, identity.user_id, body.name, body.kind))

    @router.delete("/widgets/{widget_id}", status_code=status.HTTP_204_NO_CONTENT)
    async def delete_widget(widget_id: str, identity: IdentityDep) -> None:
        await service.delete(widget_id, identity.user_id)

    return router
