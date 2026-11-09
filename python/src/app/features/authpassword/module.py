"""Registers the password slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.authpassword.http import build_router
from app.features.authpassword.usecase import PasswordService


def register(app: FastAPI, service: PasswordService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
