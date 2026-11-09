// Features authpassword queries — the sqlc analog (Kysely typed calls).
// Tables: users, password_resets, sessions. The minimal users view carries
// only the columns these flows touch.

import { sql, type ColumnType, type Kysely } from "kysely";

// ColumnType<T, I|undefined, U|undefined>: selectable, insert-optional
// (DB default), update-optional.
type Opt<T> = ColumnType<T, T | undefined, T | undefined>;

export interface UsersTable {
  id: string;
  email: string;
  password: string;
  first_name: Opt<string>;
  language: Opt<string>;
  is_active: Opt<boolean>;
  is_verified: Opt<boolean>;
  updated_at: ColumnType<Date, string | undefined, Date | undefined>;
}

export interface PasswordResetsTable {
  id: string;
  user_id: string;
  email: string;
  token: string;
  expires_at: Date;
  is_used: ColumnType<boolean, boolean | undefined, boolean>;
  attempts: ColumnType<number, number | undefined, number>;
  created_at: ColumnType<Date, string | undefined, never>;
}

export interface SessionsTable {
  id: string;
  user_id: string;
  is_active: Opt<boolean>;
}

export interface Database {
  users: UsersTable;
  password_resets: PasswordResetsTable;
  sessions: SessionsTable;
}

// passCountResetsSince :one
export const passCountResetsSince = async (
  db: Kysely<Database>,
  p: { email: string; since: Date },
): Promise<number> =>
  (
    await db
      .selectFrom("password_resets")
      .select((eb) => eb.fn.count<number>("id").as("n"))
      .where("email", "=", p.email)
      .where("created_at", ">=", p.since)
      .executeTakeFirst()
  )?.n ?? 0;

// passGetUserByEmail :one — forgot/reset projection (password not selected).
export const passGetUserByEmail = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("users")
    .select(["id", "email", "first_name", "language", "is_active", "is_verified"])
    .where("email", "=", email)
    .executeTakeFirst()) ?? null;

// passGetUserByID :one — change-flow projection (password selected,
// first_name not — sqlc zero-value parity).
export const passGetUserByID = async (db: Kysely<Database>, id: string) =>
  (await db
    .selectFrom("users")
    .select(["id", "email", "password", "language"])
    .where("id", "=", id)
    .executeTakeFirst()) ?? null;

// passCreateReset :exec
export const passCreateReset = (
  db: Kysely<Database>,
  p: { id: string; userId: string; email: string; token: string; expiresAt: Date },
): Promise<unknown> =>
  db
    .insertInto("password_resets")
    .values({
      id: p.id,
      user_id: p.userId,
      email: p.email,
      token: p.token,
      expires_at: p.expiresAt,
    })
    .execute();

// passLatestReset :one — latest code for the email (compared in TS over a
// constant-time primitive): wrong codes bump attempts, so no oracle.
export const passLatestReset = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("password_resets")
    .select(["id", "user_id", "email", "token", "expires_at", "is_used", "attempts"])
    .where("email", "=", email)
    .orderBy("created_at", "desc")
    .limit(1)
    .executeTakeFirst()) ?? null;

// passMarkResetUsed :execrows — atomic consume: the guards make the code
// single-use even when two requests race it (loser affects 0 rows → 409).
export const passMarkResetUsed = async (
  db: Kysely<Database>,
  p: { id: string; token: string },
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("password_resets")
        .set({ is_used: true })
        .where("id", "=", p.id)
        .where("is_used", "=", false)
        .where("token", "=", p.token)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// passBumpReset :execrows — failed attempt bookkeeping: after maxCodeAttempts
// wrong guesses the code reads as invalid (brute-force cap).
export const passBumpReset = (db: Kysely<Database>, id: string): Promise<unknown> =>
  db
    .updateTable("password_resets")
    .set({ attempts: sql<number>`attempts + 1` })
    .where("id", "=", id)
    .execute();

// passSetPasswordAndVerify :exec — reset auto-verifies the account.
export const passSetPasswordAndVerify = (
  db: Kysely<Database>,
  p: { id: string; password: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ password: p.password, is_verified: true, updated_at: sql`now()` })
    .where("id", "=", p.id)
    .execute();

// passSetPassword :exec
export const passSetPassword = (
  db: Kysely<Database>,
  p: { id: string; password: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ password: p.password, updated_at: sql`now()` })
    .where("id", "=", p.id)
    .execute();

// passRevokeAllSessions :exec
export const passRevokeAllSessions = (db: Kysely<Database>, userId: string): Promise<unknown> =>
  db
    .updateTable("sessions")
    .set({ is_active: false })
    .where("user_id", "=", userId)
    .where("is_active", "=", true)
    .execute();

// passRevokeOtherSessions :exec — the current session is tracked by the
// access token's jti == session id.
export const passRevokeOtherSessions = (
  db: Kysely<Database>,
  p: { userId: string; sessionId: string },
): Promise<unknown> =>
  db
    .updateTable("sessions")
    .set({ is_active: false })
    .where("user_id", "=", p.userId)
    .where("is_active", "=", true)
    .where("id", "!=", p.sessionId)
    .execute();

// Row-shape aliases for the postgres adapter mappers.
export type PassUserEmailSel = Exclude<Awaited<ReturnType<typeof passGetUserByEmail>>, null>;
export type PassUserIdSel = Exclude<Awaited<ReturnType<typeof passGetUserByID>>, null>;
export type PassResetSel = Exclude<Awaited<ReturnType<typeof passLatestReset>>, null>;
