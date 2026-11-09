"""All SQL for the social-login slice."""

from __future__ import annotations

from sqlalchemy import text

_USER_COLUMNS = """
    id, email, role, first_name, last_name, language,
    is_active, is_blocked, is_verified, google_id, github_id
"""

FIND_BY_GOOGLE = text(
    f"SELECT {_USER_COLUMNS} FROM users WHERE google_id = :subject"  # noqa: S608
)

FIND_BY_GITHUB = text(
    f"SELECT {_USER_COLUMNS} FROM users WHERE github_id = :subject"  # noqa: S608
)

FIND_BY_EMAIL = text(
    f"SELECT {_USER_COLUMNS} FROM users WHERE email = :email"  # noqa: S608
)

FIND_BY_ID = text(
    f"SELECT {_USER_COLUMNS} FROM users WHERE id = :user_id"  # noqa: S608
)

INSERT_USER = text(
    """
    INSERT INTO users (id, email, password_hash, role, first_name, last_name,
                       language, is_active, is_blocked, is_verified, google_id,
                       github_id, profile_picture, created_at, retry_count)
    VALUES (:id, :email, :password_hash, :role, :first_name, :last_name,
            :language, true, false, true, :google_id, :github_id, :picture,
            :created_at, 0)
    """
)

LINK_GOOGLE = text(
    """
    UPDATE users
    SET google_id = :subject, is_verified = true,
        profile_picture = COALESCE(NULLIF(profile_picture, ''), :picture),
        updated_at = now()
    WHERE id = :user_id
    """
)

LINK_GITHUB = text(
    """
    UPDATE users
    SET github_id = :subject, is_verified = true,
        profile_picture = COALESCE(NULLIF(profile_picture, ''), :picture),
        updated_at = now()
    WHERE id = :user_id
    """
)

INSERT_SESSION = text(
    """
    INSERT INTO sessions (id, user_id, refresh_token, ip, user_agent, is_active,
                          created_at, expires_at)
    VALUES (:id, :user_id, :refresh_token, :ip, :user_agent, true,
            :created_at, :expires_at)
    """
)
