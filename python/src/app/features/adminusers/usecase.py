"""Admin user list, fetch, guarded update, and whitelisted upgrade."""

from __future__ import annotations

import json

from app.features.adminusers.domain import AdminUser, Patch, UpgradeResult
from app.features.adminusers.ports import AdminUserStore
from app.platform.authx import Identity
from app.shared.apperr import Code, E
from app.shared.authz import can_manage_subordinate
from app.shared.rolex import Role


class AdminUserService:
    """Every mutation is audited; every update outranks its target."""

    def __init__(self, store: AdminUserStore, allowed_admin_emails: list[str]) -> None:
        self._store = store
        self._allowed = {email.strip().lower() for email in allowed_admin_emails if email}

    async def list(self, limit: int, offset: int) -> tuple[list[AdminUser], int]:
        """One clamped page plus the total count."""
        rows = await self._store.list(limit, offset)
        return rows, await self._store.count()

    async def get(self, user_id: str) -> AdminUser:
        """Fetch or 404."""
        user = await self._store.get(user_id)
        if user is None:
            msg = "user not found"
            raise E(Code.NOT_FOUND, msg, 404)
        return user

    async def update(
        self,
        actor: Identity,
        user_id: str,
        patch: Patch,
        ip: str,
        user_agent: str,
    ) -> AdminUser:
        """Validate, rank-check, patch, then audit."""
        if patch.is_active is None and patch.is_blocked is None and patch.role is None:
            msg = "empty update"
            raise E(Code.VALIDATION_FAILED, msg, 422)
        target = await self.get(user_id)
        if not can_manage_subordinate(actor.role, target.role):
            msg = "you do not outrank this user"
            raise E(Code.FORBIDDEN, msg, 403)
        if actor.user_id == user_id and patch.role is not None:
            msg = "you cannot change your own role"
            raise E(Code.FORBIDDEN, msg, 403)
        updated = await self._store.update(user_id, patch)
        if updated is None:
            msg = "user not found"
            raise E(Code.NOT_FOUND, msg, 404)
        await self._store.write_audit(
            actor.user_id,
            "admin_user.update",
            "users",
            user_id,
            json.dumps(
                {
                    "is_active": patch.is_active,
                    "is_blocked": patch.is_blocked,
                    "role": patch.role.value if patch.role is not None else None,
                }
            ),
            ip,
            user_agent,
        )
        return updated

    async def upgrade_to_admin(self, email: str) -> UpgradeResult:
        """The bootstrap route: only a whitelisted email may self-promote."""
        normalized = email.strip().lower()
        if normalized not in self._allowed:
            msg = "email is not authorized to upgrade to admin"
            raise E(Code.FORBIDDEN, msg, 403)
        user = await self._store.get_by_email(normalized)
        if user is None:
            msg = "user not found"
            raise E(Code.NOT_FOUND, msg, 404)
        if user.role is Role.ADMIN:
            return UpgradeResult(email=normalized, message="already an admin", upgraded=False)
        await self._store.upgrade_to_admin(user.id)
        await self._store.write_audit("", "auth.upgrade_to_admin", "users", user.id, "{}", "", "")
        return UpgradeResult(email=normalized, message="upgraded to admin", upgraded=True)
