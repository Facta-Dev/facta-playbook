"""Pure authorization rules shared by the admin and identity slices."""

from __future__ import annotations

from app.shared.rolex import Role, at_least, rank


def can_manage(actor: Role, target: Role) -> bool:
    """Admins manage everyone; moderators manage user and guest accounts."""
    if actor is Role.ADMIN:
        return True
    if actor is Role.MODERATOR:
        return rank(target) < rank(Role.MODERATOR)
    return False


def can_manage_subordinate(actor: Role, target: Role) -> bool:
    """A manager must outrank the target and be at least a moderator."""
    return at_least(actor, Role.MODERATOR) and rank(actor) > rank(target)


def can_manage_self(actor_id: str, target_id: str) -> bool:
    """A user always manages their own profile."""
    return actor_id == target_id
