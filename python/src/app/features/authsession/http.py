"""Session routes, including the admin session views."""

from __future__ import annotations

from datetime import datetime
from typing import Annotated

from fastapi import APIRouter, Depends, Query, Request, status
from pydantic import BaseModel, StringConstraints

from app.features.authsession.domain import Account, SessionView, TokenPair
from app.features.authsession.usecase import SessionService
from app.platform.authx import Identity, IdentityDep, require_role
from app.platform.envelope import client_ip, user_agent
from app.shared.rolex import Role

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
Admin = Annotated[Identity, Depends(require_role(Role.ADMIN))]


class LoginRequest(BaseModel):
    email: Email
    password: Password


class TokenResponse(BaseModel):
    access_token: str
    refresh_token: str


class RefreshRequest(BaseModel):
    refresh_token: Annotated[str, StringConstraints(min_length=32, max_length=256)]


class LogoutRequest(BaseModel):
    refresh_token: Annotated[str, StringConstraints(min_length=32, max_length=256)]


class AccountResponse(BaseModel):
    id: str
    email: str
    role: str
    first_name: str
    last_name: str
    language: str
    last_login: datetime | None


class UpdateMeRequest(BaseModel):
    first_name: Annotated[str | None, StringConstraints(max_length=100)] = None
    last_name: Annotated[str | None, StringConstraints(max_length=100)] = None
    language: Annotated[str | None, StringConstraints(max_length=10)] = None


class SessionResponse(BaseModel):
    id: str
    ip: str
    user_agent: str
    created_at: datetime
    expires_at: datetime
    current: bool


class RevokeResponse(BaseModel):
    revoked: int


class AnomalyResponse(BaseModel):
    user_id: str
    email: str
    distinct_ips: int
    sessions: int


def _account(account: Account) -> AccountResponse:
    return AccountResponse(
        id=account.id,
        email=account.email,
        role=account.role.value,
        first_name=account.first_name,
        last_name=account.last_name,
        language=account.language,
        last_login=account.last_login,
    )


def _session(view: SessionView) -> SessionResponse:
    return SessionResponse(
        id=view.id,
        ip=view.ip,
        user_agent=view.user_agent,
        created_at=view.created_at,
        expires_at=view.expires_at,
        current=view.current,
    )


def _pair(pair: TokenPair) -> TokenResponse:
    return TokenResponse(access_token=pair.access_token, refresh_token=pair.refresh_token)


def build_router(service: SessionService) -> APIRouter:
    """Routes only translate; every decision lives in ``SessionService``."""
    router = APIRouter(tags=["auth"])

    @router.post("/auth/login")
    async def login(body: LoginRequest, request: Request) -> TokenResponse:
        pair = await service.login(
            body.email, body.password, client_ip(request), user_agent(request)
        )
        return _pair(pair)

    @router.post("/auth/refresh")
    async def refresh(body: RefreshRequest) -> TokenResponse:
        return _pair(await service.refresh(body.refresh_token))

    @router.post("/auth/logout", status_code=status.HTTP_204_NO_CONTENT)
    async def logout(body: LogoutRequest) -> None:
        await service.logout(body.refresh_token)

    @router.get("/auth/me")
    async def me(identity: IdentityDep) -> AccountResponse:
        return _account(await service.me(identity))

    @router.patch("/auth/me")
    async def update_me(body: UpdateMeRequest, identity: IdentityDep) -> AccountResponse:
        account = await service.update_me(identity, body.first_name, body.last_name, body.language)
        return _account(account)

    @router.get("/auth/sessions")
    async def sessions(
        identity: IdentityDep,
        refresh_token: Annotated[str, Query(max_length=256)] = "",
    ) -> list[SessionResponse]:
        rows = await service.sessions(identity, refresh_token)
        return [_session(row) for row in rows]

    @router.get("/auth/sessions/current")
    async def current_session(
        identity: IdentityDep,
        refresh_token: Annotated[str, Query(max_length=256)] = "",
    ) -> SessionResponse | None:
        rows = await service.sessions(identity, refresh_token)
        for row in rows:
            if row.current:
                return _session(row)
        if rows:
            return _session(rows[0])
        return None

    @router.post("/auth/sessions/revoke-current")
    async def revoke_current(body: RefreshRequest) -> RevokeResponse:
        revoked = await service.revoke_current(body.refresh_token)
        return RevokeResponse(revoked=1 if revoked else 0)

    @router.post("/auth/sessions/revoke-all")
    async def revoke_all(identity: IdentityDep) -> RevokeResponse:
        return RevokeResponse(revoked=await service.revoke_all(identity))

    @router.get("/admin/auth-session/users/{user_id}/sessions")
    async def admin_sessions(user_id: str, _admin: Admin) -> list[SessionResponse]:
        rows = await service.admin_sessions(user_id)
        return [_session(row) for row in rows]

    @router.post("/admin/auth-session/users/{user_id}/revoke-all")
    async def admin_revoke_all(user_id: str, _admin: Admin) -> RevokeResponse:
        return RevokeResponse(revoked=await service.admin_revoke_all(user_id))

    @router.get("/admin/auth-session/anomalies")
    async def anomalies(_admin: Admin) -> list[AnomalyResponse]:
        rows = await service.anomalies()
        return [
            AnomalyResponse(
                user_id=row.user_id,
                email=row.email,
                distinct_ips=row.distinct_ips,
                sessions=row.sessions,
            )
            for row in rows
        ]

    return router
