// 0001_init — initial schema: the identity core (users, sessions, email
// codes, outbox, settings, audit) + the widgets example table.
// ponytail: replace the `widgets` example with your domain entities; keep
// the identity tables unless you know why not.
// Parity note: PKs are UUIDs; roles are TEXT + CHECK (not native PG enums)
// so down-migrations and typed queries stay trivial.

/** @param {import('node-pg-migrate').MigrationBuilder} pgm */
export const up = (pgm) => {
  pgm.sql(`
CREATE TABLE users (
    id              UUID PRIMARY KEY,
    email           VARCHAR(255) NOT NULL UNIQUE,
    first_name      VARCHAR(100) NOT NULL DEFAULT '',
    last_name       VARCHAR(100) NOT NULL DEFAULT '',
    role            VARCHAR(20)  NOT NULL DEFAULT 'USER'
        CHECK (role IN ('ADMIN', 'MODERATOR', 'USER', 'GUEST')),
    user_type       VARCHAR(20)  NOT NULL DEFAULT 'COMPANY'
        CHECK (user_type IN ('ADMIN', 'COMPANY', 'USER')),
    is_active       BOOLEAN      NOT NULL DEFAULT TRUE,
    is_blocked      BOOLEAN      NOT NULL DEFAULT FALSE,
    is_verified     BOOLEAN      NOT NULL DEFAULT FALSE,
    retry_count     INTEGER      NOT NULL DEFAULT 0,
    password        VARCHAR(255) NOT NULL,
    language        VARCHAR(10)  NOT NULL DEFAULT 'en',
    profile_picture VARCHAR(255),
    phone_number    VARCHAR(20),
    address         TEXT,
    postal_code     VARCHAR(20),
    subscription    VARCHAR(50),
    google_id       VARCHAR(255) UNIQUE,
    github_id       VARCHAR(255) UNIQUE,
    last_login      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT now()
);
-- Admin list is paginated newest-first (limit/offset).
CREATE INDEX idx_users_created_at ON users (created_at DESC);

-- One durable session row per issued refresh token; rotation deactivates it.
-- The access token is a short-lived JWT and is never stored.
CREATE TABLE sessions (
    id            UUID PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token VARCHAR(128) NOT NULL UNIQUE,
    ip_address    VARCHAR(50),
    user_agent    VARCHAR(500),
    is_active     BOOLEAN     NOT NULL DEFAULT TRUE,
    last_activity TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Revoke-all and touch queries filter on active sessions.
CREATE INDEX idx_sessions_user_active ON sessions (user_id) WHERE is_active;

-- 6-digit numeric codes; resend mints a new row (no unique on token).
CREATE TABLE email_verifications (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      VARCHAR(8)  NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL, -- 24h
    is_used    BOOLEAN     NOT NULL DEFAULT FALSE,
    attempts   INTEGER     NOT NULL DEFAULT 0, -- wrong-code guesses (brute-force cap)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_email_verifications_user
    ON email_verifications (user_id, created_at DESC);

-- 6-digit numeric codes; 30-minute expiry; reset auto-verifies the email.
-- \`email\` keeps the rate limit (3 per 5 min) honest even when the user is
-- unknown, and lets forgot-password return 204 without leaking existence.
CREATE TABLE password_resets (
    id         UUID PRIMARY KEY,
    user_id    UUID REFERENCES users(id) ON DELETE CASCADE,
    email      VARCHAR(255) NOT NULL,
    token      VARCHAR(8)   NOT NULL,
    expires_at TIMESTAMPTZ  NOT NULL, -- 30m
    is_used    BOOLEAN      NOT NULL DEFAULT FALSE,
    attempts   INTEGER      NOT NULL DEFAULT 0, -- wrong-code guesses (brute-force cap)
    created_at TIMESTAMPTZ  NOT NULL DEFAULT now()
);
CREATE INDEX idx_password_resets_user
    ON password_resets (user_id, created_at DESC);
CREATE INDEX idx_password_resets_email_recent
    ON password_resets (email, created_at);

-- The "email with job" system: an outbox table drained by an in-process
-- worker (SELECT ... FOR UPDATE SKIP LOCKED).
CREATE TABLE email_jobs (
    id           UUID PRIMARY KEY,
    to_email     VARCHAR(255) NOT NULL,
    template_key VARCHAR(100) NOT NULL,
    payload      JSONB        NOT NULL DEFAULT '{}',
    status       VARCHAR(10)  NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'sending', 'sent', 'failed')),
    retry_count  INTEGER  NOT NULL DEFAULT 0,
    max_retries  INTEGER  NOT NULL DEFAULT 3,
    last_error   TEXT,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_email_jobs_queued
    ON email_jobs (scheduled_at) WHERE status = 'queued';

-- System configuration (key/value + sensitivity flag).
CREATE TABLE settings (
    id           UUID PRIMARY KEY,
    key          VARCHAR(100) NOT NULL UNIQUE,
    value        TEXT,
    is_sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    description  TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by   UUID REFERENCES users(id) ON DELETE SET NULL
);

-- Who did what (actor, action, entity, payload, request meta).
CREATE TABLE audit_logs (
    id            UUID PRIMARY KEY,
    actor_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    action        VARCHAR(100) NOT NULL,
    entity        VARCHAR(100) NOT NULL,
    entity_id     VARCHAR(100),
    payload       JSONB,
    ip_address    VARCHAR(45),
    user_agent    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_logs_created_at ON audit_logs (created_at DESC);
CREATE INDEX idx_audit_logs_actor ON audit_logs (actor_user_id);

-- The widgets example slice, mirrored from the backend-rust template so the
-- factory vocabulary matches. owner_user_id ties rows to the auth model.
CREATE TABLE widgets (
    id            UUID PRIMARY KEY,
    name          TEXT  NOT NULL,
    kind          TEXT  NOT NULL DEFAULT 'sprocket'
        CHECK (kind IN ('sprocket', 'cog', 'flange')),
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_widgets_owner ON widgets (owner_user_id);
CREATE INDEX idx_widgets_created_at ON widgets (created_at DESC);
`);
};

/** @param {import('node-pg-migrate').MigrationBuilder} pgm */
export const down = (pgm) => {
  pgm.sql(`
DROP TABLE IF EXISTS widgets;
DROP TABLE IF EXISTS audit_logs;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS email_jobs;
DROP TABLE IF EXISTS password_resets;
DROP TABLE IF EXISTS email_verifications;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;
`);
};
