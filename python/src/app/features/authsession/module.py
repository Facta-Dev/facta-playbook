"""Registers the session slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.authsession.http import build_router
from app.features.authsession.usecase import SessionService


def register(app: FastAPI, service: SessionService) -> None:
    app.include_router(build_router(service), prefix="/api/v1")
