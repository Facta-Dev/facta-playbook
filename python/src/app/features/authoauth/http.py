"""Social-login routes: JSON for Google, redirects for GitHub."""

from __future__ import annotations

from typing import Annotated
from urllib.parse import urlencode

from fastapi import APIRouter, Query, Request
from fastapi.responses import RedirectResponse
from pydantic import BaseModel, StringConstraints

from app.features.authoauth.usecase import OAuthService
from app.platform.authx import IdentityDep
from app.platform.envelope import client_ip, user_agent
from app.shared.apperr import AppError


class GoogleRequest(BaseModel):
    credential: Annotated[str, StringConstraints(min_length=1, max_length=8192)]


class TokenResponse(BaseModel):
    access_token: str
    refresh_token: str


class ConnectResponse(BaseModel):
    url: str


def _error_redirect(frontend: str, code: str, description: str) -> RedirectResponse:
    query = urlencode({"error": code, "error_description": description})
    return RedirectResponse(f"{frontend}?{query}", status_code=302)


def build_router(service: OAuthService) -> APIRouter:
    """Google answers JSON; GitHub's login answers an envelope when disabled,
    and its callback always redirects (tokens or an error query)."""
    router = APIRouter(tags=["auth"])

    @router.post("/auth/google")
    async def google(body: GoogleRequest, request: Request) -> TokenResponse:
        pair = await service.google_login(body.credential, client_ip(request), user_agent(request))
        return TokenResponse(access_token=pair.access_token, refresh_token=pair.refresh_token)

    @router.get("/auth/github/login")
    async def github_login(
        state: Annotated[str, Query(max_length=512)] = "",
    ) -> RedirectResponse:
        return RedirectResponse(await service.github_login_url(state), status_code=302)

    @router.get("/auth/github/callback")
    async def github_callback(
        request: Request,
        code: Annotated[str, Query(max_length=512)] = "",
        state: Annotated[str, Query(max_length=4096)] = "",
        error: Annotated[str, Query(max_length=200)] = "",
        error_description: Annotated[str, Query(max_length=500)] = "",
    ) -> RedirectResponse:
        frontend = service.frontend_success
        if error or not code:
            return _error_redirect(frontend, error or "auth.no_code", error_description)
        try:
            if service.is_link_state(state):
                await service.github_link(state, code)
                return RedirectResponse(frontend, status_code=302)
            pair = await service.github_login(code, client_ip(request), user_agent(request))
        except AppError as exc:
            return _error_redirect(frontend, exc.code.value, exc.detail)
        fragment = urlencode(
            {"access_token": pair.access_token, "refresh_token": pair.refresh_token}
        )
        return RedirectResponse(f"{frontend}#{fragment}", status_code=302)

    @router.get("/auth/github/connect-url")
    async def github_connect_url(identity: IdentityDep) -> ConnectResponse:
        return ConnectResponse(url=await service.github_connect_url(identity.user_id))

    return router
