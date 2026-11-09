-- authoauth: google one-tap + github oauth flows over users + sessions.
-- name: AuthoauthGetUserByGoogleID :one
SELECT id, email, first_name, last_name, role, user_type, language,
       is_active, is_blocked, is_verified, profile_picture, google_id, github_id
FROM users WHERE google_id = $1;

-- name: AuthoauthGetUserByGithubID :one
SELECT id, email, first_name, last_name, role, user_type, language,
       is_active, is_blocked, is_verified, profile_picture, google_id, github_id
FROM users WHERE github_id = $1;

-- name: AuthoauthGetUserByEmail :one
SELECT id, email, first_name, last_name, role, user_type, language,
       is_active, is_blocked, is_verified, profile_picture, google_id, github_id
FROM users WHERE email = $1;

-- name: AuthoauthGetUserByID :one
SELECT id, email, first_name, last_name, role, user_type, language,
       is_active, is_blocked, is_verified, profile_picture, google_id, github_id
FROM users WHERE id = $1;

-- name: AuthoauthCreateUser :exec
-- Social provisioning: role/language default SQL-side ('USER'/'en'), the
-- email arrives provider-verified, and the password is an unusable secret.
INSERT INTO users (id, email, first_name, last_name, password, user_type,
                   is_verified, profile_picture, google_id, github_id)
VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8, $9);

-- name: AuthoauthLinkGoogle :exec
-- Provider proves the email: verify it; fill the picture only when empty
-- (COALESCE keeps any existing non-empty value).
UPDATE users
SET google_id = $2,
    is_verified = true,
    profile_picture = COALESCE(NULLIF(profile_picture, ''), sqlc.arg(picture)),
    updated_at = now()
WHERE id = $1;

-- name: AuthoauthLinkGithub :exec
UPDATE users
SET github_id = $2,
    is_verified = true,
    profile_picture = COALESCE(NULLIF(profile_picture, ''), sqlc.arg(picture)),
    updated_at = now()
WHERE id = $1;

-- name: AuthoauthGithubOnly :exec
-- Link flow: attach the github id only, no verification side effects.
UPDATE users SET github_id = $2, updated_at = now() WHERE id = $1;

-- name: AuthoauthSaveGithubLogin :exec
-- Login-flow identity refresh; the noreply email upgrade decision is made
-- in Go and lands here as the new email value.
UPDATE users
SET email = $2,
    is_verified = true,
    profile_picture = COALESCE(NULLIF(profile_picture, ''), sqlc.arg(picture)),
    updated_at = now()
WHERE id = $1;

-- name: AuthoauthCreateSession :exec
INSERT INTO sessions (id, user_id, refresh_token, ip_address, user_agent, expires_at)
VALUES ($1, $2, $3, $4, $5, $6);