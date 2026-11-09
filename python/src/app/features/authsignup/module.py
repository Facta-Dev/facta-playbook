"""Registers the signup slice on the composition root."""

from __future__ import annotations

from fastapi import FastAPI

from app.features.authsignup.http import build_router
from app.features.authsignup.usecase import SignupService


def register(app: FastAPI, service: SignupService) -> None:
    """Mount the slice's routes; the caller wires the service."""
    app.include_router(build_router(service), prefix="/api/v1")
