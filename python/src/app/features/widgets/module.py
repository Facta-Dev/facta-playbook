"""Registers the widget slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.widgets.http import build_router
from app.features.widgets.usecase import WidgetService


def register(app: FastAPI, service: WidgetService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
