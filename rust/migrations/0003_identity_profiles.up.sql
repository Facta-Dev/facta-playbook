-- 0003_identity_profiles.up.sql — one-to-one profile tables keyed by user_id
-- (UUID PK/FK, not INTEGER). companies profiles COMPANY accounts;
-- users_profile profiles USER accounts; ADMIN carries no profile row. Only
-- the signup slice seeds them (COMPANY -> companies, USER -> users_profile,
-- name fields from first+last); the OAuth slice provisions bare user rows
-- without a profile.

CREATE TABLE companies (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    company_name        VARCHAR(255) NOT NULL DEFAULT '',
    company_website     VARCHAR(255) NOT NULL DEFAULT '',
    industry            VARCHAR(100) NOT NULL DEFAULT '',
    tax_id              VARCHAR(50)  NOT NULL DEFAULT '',
    registration_number VARCHAR(50)  NOT NULL DEFAULT '',
    address_line1       VARCHAR(255) NOT NULL DEFAULT '',
    address_line2       VARCHAR(255) NOT NULL DEFAULT '',
    city                VARCHAR(100) NOT NULL DEFAULT '',
    state               VARCHAR(100) NOT NULL DEFAULT '',
    postal_code         VARCHAR(20)  NOT NULL DEFAULT '',
    country             VARCHAR(100) NOT NULL DEFAULT '',
    phone               VARCHAR(20)  NOT NULL DEFAULT '',
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE users_profile (
    user_id       UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    full_name     VARCHAR(255) NOT NULL DEFAULT '',
    bio           TEXT         NOT NULL DEFAULT '',
    avatar_url    VARCHAR(500),
    date_of_birth DATE,
    phone         VARCHAR(20)  NOT NULL DEFAULT '',
    address_line1 VARCHAR(255) NOT NULL DEFAULT '',
    address_line2 VARCHAR(255) NOT NULL DEFAULT '',
    city          VARCHAR(100) NOT NULL DEFAULT '',
    state         VARCHAR(100) NOT NULL DEFAULT '',
    postal_code   VARCHAR(20)  NOT NULL DEFAULT '',
    country       VARCHAR(100) NOT NULL DEFAULT '',
    occupation    VARCHAR(100) NOT NULL DEFAULT '',
    preferences   JSONB        NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);