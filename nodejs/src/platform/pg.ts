// Platform pg owns the Postgres connection: pool bootstrap and the readyz
// probe. Adapters in the feature slices share this pool.

import pg from "pg";
import type { Config } from "#platform/config.js";

// newPool opens a pg pool from the database URL and verifies it with a ping.
export const newPool = async (cfg: Config): Promise<pg.Pool> => {
  const pool = new pg.Pool({
    connectionString: cfg.databaseUrl,
    max: cfg.dbMaxConns,
  });
  try {
    await pool.query("SELECT 1");
  } catch (err) {
    await pool.end();
    throw err;
  }
  return pool;
};

// readyz pings the database inside the timeout; the caller answers 503 when
// it fails (SELECT-1 semantics via the pool's connection check).
export const readyz = async (pool: pg.Pool, timeoutMs: number): Promise<void> => {
  let timer: NodeJS.Timeout | undefined;
  try {
    await Promise.race([
      pool.query("SELECT 1"),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error("database not ready")), timeoutMs);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
};
