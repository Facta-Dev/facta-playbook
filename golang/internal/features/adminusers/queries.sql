-- adminusers: paginated user administration + audit trail.
-- name: AdminListUsers :many
SELECT id, email, first_name, last_name, role, user_type, is_active, is_blocked, is_verified, language, created_at
FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2;

-- name: AdminCountUsers :one
SELECT count(*)::int AS n FROM users;

-- name: AdminGetUser :one
SELECT id, email, first_name, last_name, role, user_type, is_active, is_blocked, is_verified, language, created_at, last_login
FROM users WHERE id = $1;

-- name: AdminGetUserByEmail :one
SELECT id, email, first_name, last_name, role, user_type, is_active, is_blocked, is_verified, language, created_at, last_login
FROM users WHERE email = $1;

-- name: AdminUpgradeToAdmin :exec
-- Bootstrap whitelist upgrade (reference upgrade_to_admin parity): both the
-- role and the user_type move to ADMIN so RBAC and the profile rules agree.
UPDATE users SET role = 'ADMIN', user_type = 'ADMIN', updated_at = now() WHERE id = $1;

-- name: AdminUpdateUser :exec
UPDATE users SET
  is_active  = COALESCE(sqlc.narg('is_active'),  is_active),
  is_blocked = COALESCE(sqlc.narg('is_blocked'), is_blocked),
  role       = COALESCE(sqlc.narg('role'),       role),
  updated_at = now()
WHERE id = sqlc.arg('id');

-- name: AdminWriteAudit :exec
INSERT INTO audit_logs (id, actor_user_id, action, entity, entity_id, payload, ip_address, user_agent)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8);
