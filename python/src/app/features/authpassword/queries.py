"""All SQL for password reset and change."""

from __future__ import annotations

from sqlalchemy import text

FIND_ACCOUNT = text("SELECT id, email, password_hash FROM users WHERE email = :email")

RECENT_RESETS = text(
    "SELECT count(*) AS n FROM password_resets WHERE email = :email AND created_at >= :since"
)

INSERT_RESET = text(
    """
    INSERT INTO password_resets (id, email, code, expires_at, attempts, created_at)
    VALUES (:id, :email, :code, :expires_at, 0, :created_at)
    """
)

FIND_RESET = text(
    """
    SELECT id, email, code, expires_at, attempts
    FROM password_resets
    WHERE email = :email
    ORDER BY created_at DESC
    LIMIT 1
    """
)

INCREMENT_ATTEMPTS = text("UPDATE password_resets SET attempts = attempts + 1 WHERE id = :id")

DELETE_RESET = text("DELETE FROM password_resets WHERE email = :email")

SET_PASSWORD = text("UPDATE users SET password_hash = :password_hash WHERE id = :user_id")

REVOKE_SESSIONS = text(
    """
    UPDATE sessions SET is_active = false
    WHERE user_id = :user_id AND is_active = true
    RETURNING id
    """
)
