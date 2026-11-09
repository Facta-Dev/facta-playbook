-- authsignup: users + user_type/profile seeding + email_verifications.
-- name: SignupGetUserByEmail :one
SELECT id, email, first_name, last_name, role, user_type, language, is_active, is_verified
FROM users WHERE email = $1;

-- name: SignupCreateUser :exec
INSERT INTO users (id, email, first_name, last_name, password, language, user_type)
VALUES ($1, $2, $3, $4, $5, $6, $7);

-- name: SignupCreateCompanyProfile :exec
-- 1:1 company profile (user_id PK, 0002_profiles): signup seeds only the
-- NOT NULL name (first + last); the rest stays column-default NULL/empty.
INSERT INTO companies (user_id, company_name)
VALUES ($1, $2);

-- name: SignupCreateUserProfile :exec
-- 1:1 personal profile (user_id PK, 0002_profiles): full_name = first + last.
INSERT INTO users_profile (user_id, full_name)
VALUES ($1, $2);

-- name: SignupCreateVerification :exec
INSERT INTO email_verifications (id, user_id, token, expires_at)
VALUES ($1, $2, $3, $4);

-- name: SignupLatestVerification :one
SELECT id, user_id, token, expires_at, is_used, attempts
FROM email_verifications
WHERE user_id = $1
ORDER BY created_at DESC
LIMIT 1;

-- name: SignupUseVerification :execrows
-- Atomic consume: the guards make the code single-use even when two requests
-- race the same code (one wins, the loser affects 0 rows and is rejected).
UPDATE email_verifications
SET is_used = true
WHERE id = $1 AND is_used = false AND token = $2;

-- name: SignupBumpVerification :execrows
-- Failed attempt bookkeeping: after maxCodeAttempts wrong guesses the code
-- reads as invalid (brute-force cap).
UPDATE email_verifications SET attempts = attempts + 1 WHERE id = $1;

-- name: SignupSetVerified :exec
UPDATE users SET is_verified = true, updated_at = now() WHERE id = $1;
