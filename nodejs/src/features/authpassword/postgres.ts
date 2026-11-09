// Features authpassword postgres adapts the Kysely database to this slice's
// Store port (the sqlc adapter's analog). The by-ID lookup keeps sqlc
// zero-value parity: first_name reads "" because the projection does not
// select it.

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import type { Reset, User } from "./domain.js";
import type { Store } from "./ports.js";
import {
  passCountResetsSince,
  passGetUserByEmail,
  passGetUserByID,
  passCreateReset,
  passLatestReset,
  passMarkResetUsed,
  passBumpReset,
  passSetPasswordAndVerify,
  passSetPassword,
  passRevokeAllSessions,
  passRevokeOtherSessions,
  type Database,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async countResetsSince(email: string, since: Date): Promise<number> {
    return passCountResetsSince(this.#db, { email, since });
  }

  async getUserByEmail(email: string): Promise<User | null> {
    const r = await passGetUserByEmail(this.#db, email);
    return r === null
      ? null
      : { id: r.id, email: r.email, firstName: r.first_name, password: "", language: r.language };
  }

  async getUserByID(id: string): Promise<User | null> {
    const r = await passGetUserByID(this.#db, id);
    return r === null
      ? null
      : { id: r.id, email: r.email, firstName: "", password: r.password, language: r.language };
  }

  async createReset(r: Reset): Promise<void> {
    await passCreateReset(this.#db, {
      id: r.id,
      userId: r.userId,
      email: r.email,
      token: r.token,
      expiresAt: r.expiresAt,
    });
  }

  async latestReset(email: string): Promise<Reset | null> {
    const r = await passLatestReset(this.#db, email);
    return r === null
      ? null
      : {
          id: r.id,
          userId: r.user_id,
          email: r.email,
          token: r.token,
          expiresAt: r.expires_at,
          used: r.is_used,
          attempts: r.attempts,
          // not selected: rate limiting counts via countResetsSince
          createdAt: new Date(0),
        };
  }

  async markResetUsed(id: string, token: string): Promise<boolean> {
    return (await passMarkResetUsed(this.#db, { id, token })) > 0;
  }

  async bumpReset(id: string): Promise<void> {
    await passBumpReset(this.#db, id);
  }

  async setPasswordAndVerify(id: string, hash: string): Promise<void> {
    await passSetPasswordAndVerify(this.#db, { id, password: hash });
  }

  async setPassword(id: string, hash: string): Promise<void> {
    await passSetPassword(this.#db, { id, password: hash });
  }

  async revokeAllSessions(userId: string): Promise<void> {
    await passRevokeAllSessions(this.#db, userId);
  }

  async revokeOtherSessions(userId: string, sessionId: string): Promise<void> {
    await passRevokeOtherSessions(this.#db, { userId, sessionId });
  }
}
