"""Registers the social-login slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.authoauth.http import build_router
from app.features.authoauth.usecase import OAuthService


def register(app: FastAPI, service: OAuthService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
