"""Consumer-owned port for admin user management."""

from __future__ import annotations

from typing import Protocol

from app.features.adminusers.domain import AdminUser, Patch


class AdminUserStore(Protocol):
    """Reads, the guarded update, and the audit trail."""

    async def list(self, limit: int, offset: int) -> list[AdminUser]:
        """One page of users, newest first."""
        ...

    async def count(self) -> int:
        """Total row count for the page envelope."""
        ...

    async def get(self, user_id: str) -> AdminUser | None:
        """One user, or ``None``."""
        ...

    async def get_by_email(self, email: str) -> AdminUser | None:
        """One user by email, or ``None``."""
        ...

    async def update(self, user_id: str, patch: Patch) -> AdminUser | None:
        """Apply the patch and return the fresh row."""
        ...

    async def upgrade_to_admin(self, user_id: str) -> None:
        """Raise the role to admin."""
        ...

    async def write_audit(
        self,
        actor_id: str,
        action: str,
        entity: str,
        entity_id: str,
        payload: str,
        ip: str,
        user_agent: str,
    ) -> None:
        """Append one audit_logs row."""
        ...
