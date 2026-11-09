"""Admin user routes and the whitelisted bootstrap upgrade."""

from __future__ import annotations

from datetime import datetime
from typing import Annotated

from fastapi import APIRouter, Depends, Query, Request
from pydantic import BaseModel, StringConstraints

from app.features.adminusers.domain import AdminUser, Patch
from app.features.adminusers.usecase import AdminUserService
from app.platform.authx import Identity, require_role
from app.platform.envelope import client_ip, pagination, user_agent
from app.shared.rolex import Role

Admin = Annotated[Identity, Depends(require_role(Role.ADMIN))]

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


class AdminUserResponse(BaseModel):
    id: str
    email: str
    first_name: str
    last_name: str
    role: str
    is_active: bool
    is_blocked: bool
    is_verified: bool
    language: str
    created_at: datetime
    last_login: datetime | None


class AdminUserListResponse(BaseModel):
    items: list[AdminUserResponse]
    total: int


class UpdateRequest(BaseModel):
    is_active: bool | None = None
    is_blocked: bool | None = None
    role: Role | None = None


class UpgradeRequest(BaseModel):
    email: Email


class UpgradeResponse(BaseModel):
    email: str
    message: str
    upgraded: bool


def _model(user: AdminUser) -> AdminUserResponse:
    return AdminUserResponse(
        id=user.id,
        email=user.email,
        first_name=user.first_name,
        last_name=user.last_name,
        role=user.role.value,
        is_active=user.is_active,
        is_blocked=user.is_blocked,
        is_verified=user.is_verified,
        language=user.language,
        created_at=user.created_at,
        last_login=user.last_login,
    )


def build_router(service: AdminUserService) -> APIRouter:
    """Routes only translate; rank checks live in the service."""
    router = APIRouter(tags=["admin"])

    @router.get("/admin/users")
    async def list_users(
        _admin: Admin,
        page: Annotated[int, Query(ge=1)] = 1,
        page_size: Annotated[int, Query(ge=1, le=100)] = 20,
    ) -> AdminUserListResponse:
        safe_page, safe_size = pagination(page, page_size)
        rows, total = await service.list(safe_size, (safe_page - 1) * safe_size)
        return AdminUserListResponse(items=[_model(row) for row in rows], total=total)

    @router.get("/admin/users/{user_id}")
    async def get_user(user_id: str, _admin: Admin) -> AdminUserResponse:
        return _model(await service.get(user_id))

    @router.patch("/admin/users/{user_id}")
    async def update_user(
        user_id: str,
        body: UpdateRequest,
        actor: Admin,
        request: Request,
    ) -> AdminUserResponse:
        updated = await service.update(
            actor,
            user_id,
            Patch(is_active=body.is_active, is_blocked=body.is_blocked, role=body.role),
            client_ip(request),
            user_agent(request),
        )
        return _model(updated)

    @router.post("/auth/upgrade-to-admin")
    async def upgrade_to_admin(body: UpgradeRequest) -> UpgradeResponse:
        result = await service.upgrade_to_admin(body.email)
        return UpgradeResponse(email=result.email, message=result.message, upgraded=result.upgraded)

    return router
