-- authpassword: forgot/validate/reset/change over users, password_resets, sessions.
-- name: PassCountResetsSince :one
SELECT count(*)::int AS n FROM password_resets WHERE email = $1 AND created_at >= $2;

-- name: PassGetUserByEmail :one
SELECT id, email, first_name, language, is_active, is_verified FROM users WHERE email = $1;

-- name: PassGetUserByID :one
SELECT id, email, password, language FROM users WHERE id = $1;

-- name: PassCreateReset :exec
INSERT INTO password_resets (id, user_id, email, token, expires_at) VALUES ($1, $2, $3, $4, $5);

-- name: PassLatestReset :one
-- Latest code for the email (token compared in Go over a constant-time
-- primitive): wrong codes bump attempts, so the code is not an oracle.
SELECT id, user_id, email, token, expires_at, is_used, attempts
FROM password_resets WHERE email = $1
ORDER BY created_at DESC LIMIT 1;

-- name: PassMarkResetUsed :execrows
-- Atomic consume: the guards make the code single-use even when two requests
-- race the same code (one wins, the loser affects 0 rows and is rejected).
UPDATE password_resets SET is_used = true
WHERE id = $1 AND is_used = false AND token = $2;

-- name: PassBumpReset :execrows
-- Failed attempt bookkeeping: after maxCodeAttempts wrong guesses the code
-- reads as invalid (brute-force cap).
UPDATE password_resets SET attempts = attempts + 1 WHERE id = $1;

-- name: PassSetPasswordAndVerify :exec
UPDATE users SET password = $2, is_verified = true, updated_at = now() WHERE id = $1;

-- name: PassSetPassword :exec
UPDATE users SET password = $2, updated_at = now() WHERE id = $1;

-- name: PassRevokeAllSessions :exec
UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active;

-- name: PassRevokeOtherSessions :exec
-- The current session is tracked by the access token's jti == session id.
UPDATE sessions SET is_active = false WHERE user_id = $1 AND is_active AND id <> $2;
