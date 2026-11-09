// Features authsignup postgres adapts the Kysely database to this slice's
// Store port. The Go template used a sqlc-generated adapter; here Kysely IS
// the typed query layer (same shape, one fewer tool).

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import { errEmailTaken } from "./domain.js";
import type { UserRow, Verification } from "./domain.js";
import type { Store } from "./ports.js";
import {
  signupBumpVerification,
  signupCreateUser,
  signupCreateCompanyProfile,
  signupCreateUserProfile,
  signupCreateVerification,
  signupGetUserByEmail,
  signupLatestVerification,
  signupSetVerified,
  signupUseVerification,
  type Database,
  type SignupUserSel,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async getUserByEmail(email: string): Promise<UserRow | null> {
    const r = await signupGetUserByEmail(this.#db, email);
    return r === null ? null : rowToUser(r);
  }

  // createUser mirrors the use-case's unique-constraint contract: a lost
  // signup race (both saw "unverified", one insert wins) answers as 409
  // AUTH_EMAIL_TAKEN instead of an unmapped 500. Postgres error code 23505
  // = unique_violation (not worth a dep for one code).
  async createUser(row: UserRow): Promise<void> {
    try {
      await signupCreateUser(this.#db, {
        id: row.id,
        email: row.email,
        firstName: row.firstName,
        lastName: row.lastName,
        password: row.passwordHash,
        language: row.language,
        userType: row.userType,
      });
    } catch (err) {
      if ((err as { code?: string }).code === "23505") {
        throw errEmailTaken;
      }
      throw err;
    }
  }

  async createCompanyProfile(userId: string, companyName: string): Promise<void> {
    await signupCreateCompanyProfile(this.#db, { userId, companyName });
  }

  async createUserProfile(userId: string, fullName: string): Promise<void> {
    await signupCreateUserProfile(this.#db, { userId, fullName });
  }

  async createVerification(v: Verification): Promise<void> {
    await signupCreateVerification(this.#db, {
      id: v.id,
      userId: v.userId,
      token: v.token,
      expiresAt: v.expiresAt,
    });
  }

  async latestVerification(userId: string): Promise<Verification | null> {
    const r = await signupLatestVerification(this.#db, userId);
    return r === null
      ? null
      : {
          id: r.id,
          userId: r.user_id,
          token: r.token,
          expiresAt: r.expires_at,
          used: r.is_used,
          attempts: r.attempts,
        };
  }

  async useVerification(vId: string, token: string): Promise<boolean> {
    return (await signupUseVerification(this.#db, { id: vId, token })) > 0;
  }

  async bumpVerification(vId: string): Promise<void> {
    await signupBumpVerification(this.#db, vId);
  }

  async setEmailVerified(userId: string): Promise<void> {
    await signupSetVerified(this.#db, userId);
  }
}

// rowToUser maps a database row onto the slice model. passwordHash is never
// selected by this slice (authsession owns password reads).
const rowToUser = (r: SignupUserSel): UserRow => ({
  id: r.id,
  email: r.email,
  firstName: r.first_name,
  lastName: r.last_name,
  passwordHash: "",
  role: r.role,
  userType: r.user_type,
  language: r.language,
  isActive: r.is_active,
  isVerified: r.is_verified,
});
