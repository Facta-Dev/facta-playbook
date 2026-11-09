-- widgets: the example CRUD, owner-scoped.
-- name: WidgetInsert :exec
INSERT INTO widgets (id, name, kind, owner_user_id) VALUES ($1, $2, $3, $4);

-- name: WidgetListByOwner :many
SELECT id, name, kind, owner_user_id, created_at
FROM widgets WHERE owner_user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3;

-- name: WidgetGetByID :one
SELECT id, name, kind, owner_user_id, created_at FROM widgets WHERE id = $1 AND owner_user_id = $2;

-- name: WidgetUpdate :exec
UPDATE widgets SET name = $3, kind = $4 WHERE id = $1 AND owner_user_id = $2;

-- name: WidgetDelete :exec
DELETE FROM widgets WHERE id = $1 AND owner_user_id = $2;
