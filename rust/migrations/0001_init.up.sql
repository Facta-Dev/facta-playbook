-- 0001_init.up.sql — initial schema. Pair with 0001_init.down.sql.
-- ponytail: replace the `widget` example table with your domain entities.
CREATE TABLE IF NOT EXISTS widgets (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL DEFAULT 'sprocket'
        CHECK (kind IN ('sprocket', 'cog', 'flange')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);