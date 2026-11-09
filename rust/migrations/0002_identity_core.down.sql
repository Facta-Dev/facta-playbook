-- 0002_identity_core.down.sql — exact inverse of 0002_identity_core.up.sql.
-- The example widgets table is NOT dropped: it ships in 0001_init.
DROP TABLE IF EXISTS audit_logs;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS email_jobs;
DROP TABLE IF EXISTS password_resets;
DROP TABLE IF EXISTS email_verifications;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;