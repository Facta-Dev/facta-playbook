-- authsession: login/refresh/logout/me over users + sessions.
-- name: SessionLoginGetUserByEmail :one
SELECT id, email, password, role, language, is_active, is_blocked, is_verified, retry_count,
       google_id, github_id
FROM users WHERE email = $1;

-- name: SessionUserByID :one
-- The me-row shape: profile + audit columns (reference MeResponse widening).
SELECT id, email, first_name, last_name, role, user_type, language,
       is_active, is_blocked, is_verified, retry_count,
       phone_number, profile_picture, subscription, address, postal_code,
       CAST(google_id IS NOT NULL AS bool) AS google_linked,
       CAST(github_id IS NOT NULL AS bool) AS github_linked,
       last_login, created_at, updated_at
FROM users WHERE id = $1;

-- name: SessionAfterFailedLogin :one
-- The counter increments SQL-side so concurrent failures cannot lose an
-- increment (a Go-side read-modify-write would); the DB computes the lock.
UPDATE users
SET retry_count = retry_count + 1,
    is_blocked = (retry_count + 1 >= sqlc.arg(lock_at)),
    updated_at = now()
WHERE id = $1
RETURNING is_blocked;

-- name: SessionAfterSuccessfulLogin :exec
UPDATE users SET retry_count = 0, last_login = now(), updated_at = now() WHERE id = $1;

-- name: SessionCreate :exec
INSERT INTO sessions (id, user_id, refresh_token, ip_address, user_agent, expires_at)
VALUES ($1, $2, $3, $4, $5, $6);

-- name: SessionGetActiveByToken :one
SELECT s.id AS session_id, s.user_id, s.refresh_token, s.expires_at, s.is_active,
       u.email, u.role, u.is_active AS user_is_active, u.is_blocked, u.is_verified
FROM sessions s JOIN users u ON u.id = s.user_id
WHERE s.refresh_token = $1 AND s.is_active AND s.expires_at > now();

-- name: SessionDeactivate :execrows
-- Atomic claim of the refresh-row (bad account state / revocation paths).
UPDATE sessions SET is_active = false WHERE id = $1 AND is_active;

-- name: SessionRotateTokens :execrows
-- In-place token rotation (reference update_tokens parity): the row keeps its
-- id and expires_at, so a session's absolute lifetime is fixed at login and
-- never slides forward; only the refresh token, ip/ua and last_activity
-- change. The WHERE guard on the old token is the compare-and-swap — a
-- replayed token can only ever win once.
UPDATE sessions
SET refresh_token = $1, ip_address = $2, user_agent = $3, last_activity = now()
WHERE id = $4 AND refresh_token = $5 AND is_active AND expires_at > now();

-- name: SessionDeactivateByTokenForUser :exec
UPDATE sessions SET is_active = false WHERE refresh_token = $1 AND user_id = $2 AND is_active;

-- name: SessionListByUser :many
SELECT id, user_id, refresh_token, ip_address, user_agent, is_active, last_activity, expires_at, created_at
FROM sessions
WHERE user_id = $1
  AND (sqlc.arg(active_only)::bool = false OR is_active)
ORDER BY last_activity DESC;

-- name: SessionCurrentByUser :one
-- The "current" session of a user: most recent active row by last_activity.
SELECT id, user_id, refresh_token, ip_address, user_agent, is_active, last_activity, expires_at, created_at
FROM sessions
WHERE user_id = $1 AND is_active
ORDER BY last_activity DESC
LIMIT 1;

-- name: SessionRevokeForUser :execrows
-- Compare-and-swap on is_active scoped to the owning user: revoking someone
-- else's session id affects 0 rows and reads as not-revoked.
UPDATE sessions SET is_active = false
WHERE user_id = $1 AND id = $2 AND is_active;

-- name: SessionRevokeAllForUser :execrows
UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active;

-- name: SessionCountExpiredActive :one
SELECT count(*)::int AS n
FROM sessions WHERE is_active AND expires_at < now();

-- name: SessionAnomalyMultiIP :many
-- Users whose active sessions within the window span more than 3 distinct
-- IPs (reference _MULTI_IP_THRESHOLD); NULL ip_address rows are not counted.
SELECT user_id, count(DISTINCT ip_address)::int AS ip_count
FROM sessions
WHERE is_active AND last_activity >= sqlc.arg(since)
GROUP BY user_id
HAVING count(DISTINCT ip_address)::int > 3;

-- name: SessionUpdateMe :execrows
-- Partial profile update (reference PUT /account/me parity): each SET entry
-- coalesces its parameter against the current value, so absent fields are
-- left unchanged; an empty pointer value sets '' (never NULL — presence is
-- expressed by the pointer, not the emptiness).
UPDATE users SET
  first_name      = COALESCE(sqlc.narg('first_name')::varchar, first_name),
  last_name       = COALESCE(sqlc.narg('last_name')::varchar, last_name),
  phone_number    = COALESCE(sqlc.narg('phone_number')::varchar, phone_number),
  profile_picture = COALESCE(sqlc.narg('profile_picture')::varchar, profile_picture),
  language        = COALESCE(sqlc.narg('language')::varchar, language),
  address         = COALESCE(sqlc.narg('address')::text, address),
  postal_code     = COALESCE(sqlc.narg('postal_code')::varchar, postal_code),
  updated_at      = now()
WHERE id = sqlc.arg('user_id');
