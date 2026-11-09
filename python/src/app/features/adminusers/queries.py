"""All SQL for admin user management."""

from __future__ import annotations

from sqlalchemy import text

_COLUMNS = """
    id, email, first_name, last_name, role, is_active, is_blocked, is_verified,
    language, created_at, last_login
"""

LIST = text(
    f"SELECT {_COLUMNS} FROM users ORDER BY created_at DESC LIMIT :limit OFFSET :offset"  # noqa: S608
)

COUNT = text("SELECT count(*) AS n FROM users")

GET = text(
    f"SELECT {_COLUMNS} FROM users WHERE id = :user_id"  # noqa: S608
)

GET_BY_EMAIL = text(
    f"SELECT {_COLUMNS} FROM users WHERE email = :email"  # noqa: S608
)

UPDATE = text(
    """
    UPDATE users
    SET is_active = COALESCE(:is_active, is_active),
        is_blocked = COALESCE(:is_blocked, is_blocked),
        role = COALESCE(:role, role),
        updated_at = now()
    WHERE id = :user_id
    RETURNING id, email, first_name, last_name, role, is_active, is_blocked,
              is_verified, language, created_at, last_login
    """
)

UPGRADE_TO_ADMIN = text("UPDATE users SET role = 'admin', updated_at = now() WHERE id = :user_id")

INSERT_AUDIT = text(
    """
    INSERT INTO audit_logs (id, actor_id, action, entity, entity_id, payload, ip,
                            user_agent, created_at)
    VALUES (:id, :actor_id, :action, :entity, :entity_id, CAST(:payload AS jsonb),
            :ip, :user_agent, now())
    """
)
