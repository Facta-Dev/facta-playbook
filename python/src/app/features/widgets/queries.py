"""All SQL for the widget slice."""

from __future__ import annotations

from sqlalchemy import text

INSERT = text(
    """
    INSERT INTO widgets (id, name, kind, owner_user_id, created_at)
    VALUES (:id, :name, :kind, :owner_user_id, :created_at)
    """
)

LIST = text(
    """
    SELECT id, name, kind, owner_user_id, created_at
    FROM widgets
    WHERE owner_user_id = :owner_id
    ORDER BY created_at DESC
    LIMIT :limit OFFSET :offset
    """
)

GET = text(
    """
    SELECT id, name, kind, owner_user_id, created_at
    FROM widgets
    WHERE id = :widget_id AND owner_user_id = :owner_id
    """
)

UPDATE = text(
    """
    UPDATE widgets
    SET name = :name, kind = :kind
    WHERE id = :widget_id AND owner_user_id = :owner_id
    RETURNING id, name, kind, owner_user_id, created_at
    """
)

DELETE = text(
    "DELETE FROM widgets WHERE id = :widget_id AND owner_user_id = :owner_id RETURNING id"
)
