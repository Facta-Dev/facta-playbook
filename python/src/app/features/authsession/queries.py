"""All SQL for the session slice."""

from __future__ import annotations

from sqlalchemy import text

_ACCOUNT_COLUMNS = """
    id, email, password_hash, role, first_name, last_name, language,
    is_active, is_blocked, is_verified, last_login
"""

FIND_ACCOUNT_BY_EMAIL = text(
    f"SELECT {_ACCOUNT_COLUMNS} FROM users WHERE email = :email"  # noqa: S608
)

FIND_ACCOUNT_BY_ID = text(
    f"SELECT {_ACCOUNT_COLUMNS} FROM users WHERE id = :user_id"  # noqa: S608
)

RECORD_LOGIN = text("UPDATE users SET last_login = :now WHERE id = :user_id")

INSERT_SESSION = text(
    """
    INSERT INTO sessions (id, user_id, refresh_token, ip, user_agent, is_active,
                          created_at, expires_at)
    VALUES (:id, :user_id, :refresh_token, :ip, :user_agent, true, :created_at, :expires_at)
    """
)

FIND_SESSION = text(
    """
    SELECT id, user_id, refresh_token, ip, user_agent, is_active, created_at, expires_at
    FROM sessions
    WHERE refresh_token = :token
    """
)

ROTATE = text(
    """
    UPDATE sessions
    SET refresh_token = :new_token, expires_at = :expires_at
    WHERE id = :id AND refresh_token = :old_token AND is_active = true
      AND expires_at > :now
    RETURNING id
    """
)

DEACTIVATE = text(
    """
    UPDATE sessions SET is_active = false
    WHERE refresh_token = :token AND is_active = true
    RETURNING id
    """
)

DEACTIVATE_ALL = text(
    """
    UPDATE sessions SET is_active = false
    WHERE user_id = :user_id AND is_active = true
    RETURNING id
    """
)

LIST_SESSIONS = text(
    """
    SELECT id, user_id, refresh_token, ip, user_agent, is_active, created_at, expires_at
    FROM sessions
    WHERE user_id = :user_id AND is_active = true AND expires_at > :now
    ORDER BY created_at DESC
    """
)

UPDATE_PROFILE = text(
    """
    UPDATE users
    SET first_name = :first_name, last_name = :last_name, language = :language
    WHERE id = :user_id
    RETURNING id, email, password_hash, role, first_name, last_name, language,
              is_active, is_blocked, is_verified, last_login
    """
)

ANOMALIES = text(
    """
    SELECT s.user_id, u.email, count(DISTINCT s.ip) AS distinct_ips,
           count(*) AS sessions
    FROM sessions s
    JOIN users u ON u.id = s.user_id
    WHERE s.is_active = true AND s.expires_at > :now
    GROUP BY s.user_id, u.email
    HAVING count(DISTINCT s.ip) > 1
    ORDER BY distinct_ips DESC
    """
)
