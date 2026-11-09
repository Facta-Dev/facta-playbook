"""All SQL for the signup slice. Only the adapter imports this module."""

from __future__ import annotations

from sqlalchemy import text

FIND_USER = text(
    """
    SELECT id, email, password_hash, role, first_name, last_name, language,
           is_verified, created_at
    FROM users
    WHERE email = :email
    """
)

INSERT_USER = text(
    """
    INSERT INTO users (id, email, password_hash, role, first_name, last_name,
                       language, is_verified, created_at, retry_count)
    VALUES (:id, :email, :password_hash, :role, :first_name, :last_name,
            :language, false, :created_at, 0)
    """
)

DELETE_CODES = text("DELETE FROM email_verifications WHERE user_id = :user_id")

INSERT_CODE = text(
    """
    INSERT INTO email_verifications (id, user_id, code, expires_at, attempts, created_at)
    VALUES (:id, :user_id, :code, :expires_at, 0, :created_at)
    """
)

FIND_CODE = text(
    """
    SELECT id, user_id, code, expires_at, attempts
    FROM email_verifications
    WHERE user_id = :user_id
    ORDER BY created_at DESC
    LIMIT 1
    """
)

INCREMENT_ATTEMPTS = text("UPDATE email_verifications SET attempts = attempts + 1 WHERE id = :id")

MARK_VERIFIED = text("UPDATE users SET is_verified = true WHERE id = :user_id")
