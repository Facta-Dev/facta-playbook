"""Signup routes: JSON in, envelope out, no SQL."""

from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter, status
from pydantic import BaseModel, StringConstraints

from app.features.authsignup.usecase import SignupService

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


class SignupRequest(BaseModel):
    email: Email
    password: Password
    first_name: Annotated[str, StringConstraints(max_length=100)] = ""
    last_name: Annotated[str, StringConstraints(max_length=100)] = ""
    language: Annotated[str, StringConstraints(max_length=10)] = "en"


class SignupResponse(BaseModel):
    user_id: str
    code_queued: bool


class VerifyRequest(BaseModel):
    email: Email
    code: Code


class VerifyResponse(BaseModel):
    verified: bool


class ResendRequest(BaseModel):
    email: Email


class ResendResponse(BaseModel):
    queued: bool


def build_router(service: SignupService) -> APIRouter:
    """Routes are thin: decode, call the use case, encode."""
    router = APIRouter(prefix="/auth", tags=["auth"])

    @router.post("/signup", status_code=status.HTTP_201_CREATED)
    async def signup(body: SignupRequest) -> SignupResponse:
        result = await service.signup(
            body.email, body.password, body.first_name, body.last_name, body.language
        )
        return SignupResponse(user_id=result.user_id, code_queued=result.code_queued)

    @router.post("/verify")
    async def verify(body: VerifyRequest) -> VerifyResponse:
        await service.verify(body.email, body.code)
        return VerifyResponse(verified=True)

    @router.post("/verify/resend", status_code=status.HTTP_202_ACCEPTED)
    async def resend(body: ResendRequest) -> ResendResponse:
        await service.resend(body.email)
        return ResendResponse(queued=True)

    return router
