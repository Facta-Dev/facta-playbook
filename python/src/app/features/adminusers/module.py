"""Registers the admin user slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.adminusers.http import build_router
from app.features.adminusers.usecase import AdminUserService


def register(app: FastAPI, service: AdminUserService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
