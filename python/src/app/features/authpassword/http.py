"""Password reset and change routes."""

from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter
from pydantic import BaseModel, StringConstraints

from app.features.authpassword.usecase import PasswordService
from app.platform.authx import IdentityDep

Email = Annotated[
    str,
    StringConstraints(
        strip_whitespace=True,
        to_lower=True,
        min_length=3,
        max_length=254,
        pattern=r"^[^@\s]+@[^@\s]+\.[^@\s]+$",
    ),
]
Password = Annotated[str, StringConstraints(min_length=8, max_length=128)]
Code = Annotated[str, StringConstraints(strip_whitespace=True, min_length=6, max_length=6)]


class ForgotRequest(BaseModel):
    email: Email


class ForgotResponse(BaseModel):
    queued: bool


class ValidateRequest(BaseModel):
    email: Email
    code: Code


class ValidateResponse(BaseModel):
    valid: bool


class ResetRequest(BaseModel):
    email: Email
    code: Code
    new_password: Password


class ResetResponse(BaseModel):
    reset: bool


class ChangeRequest(BaseModel):
    current_password: Annotated[str, StringConstraints(min_length=1, max_length=128)]
    new_password: Password


class ChangeResponse(BaseModel):
    changed: bool


def build_router(service: PasswordService) -> APIRouter:
    """Routes are thin: decode, call the use case, encode."""
    router = APIRouter(prefix="/auth", tags=["auth"])

    @router.post("/forgot-password", status_code=202)
    async def forgot(body: ForgotRequest) -> ForgotResponse:
        await service.forgot(body.email)
        return ForgotResponse(queued=True)

    @router.post("/validate-code")
    async def validate(body: ValidateRequest) -> ValidateResponse:
        await service.validate(body.email, body.code)
        return ValidateResponse(valid=True)

    @router.post("/reset-password")
    async def reset(body: ResetRequest) -> ResetResponse:
        await service.reset(body.email, body.code, body.new_password)
        return ResetResponse(reset=True)

    @router.post("/change-password")
    async def change(identity: IdentityDep, body: ChangeRequest) -> ChangeResponse:
        await service.change(identity, body.current_password, body.new_password)
        return ChangeResponse(changed=True)

    return router
