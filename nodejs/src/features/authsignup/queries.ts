// Features authsignup queries — the sqlc analog. Kysely types the schema and
// builds the same named queries as type-checked calls (the documented
// codegen delta). Column names snake_case, exactly as in migrations.

import { sql, type ColumnType, type Kysely, type Selectable } from "kysely";

// ColumnType<T, I|undefined, U|undefined>: selectable always with the SELECT
// list below, insert-optional (DB default), update-optional.
type Opt<T> = ColumnType<T, T | undefined, T | undefined>;

export interface UsersTable {
  id: string;
  email: string;
  first_name: string;
  last_name: string;
  password: string;
  role: Opt<string>;
  user_type: Opt<string>;
  language: Opt<string>;
  is_active: Opt<boolean>;
  is_verified: Opt<boolean>;
  created_at: ColumnType<Date, string | undefined, never>;
  updated_at: ColumnType<Date, string | undefined, never>;
}

export interface CompaniesTable {
  user_id: string;
  company_name: string;
  created_at: ColumnType<Date, string | undefined, never>;
}

export interface UsersProfileTable {
  user_id: string;
  full_name: string;
  created_at: ColumnType<Date, string | undefined, never>;
}

export interface EmailVerificationsTable {
  id: string;
  user_id: string;
  token: string;
  expires_at: Date;
  is_used: ColumnType<boolean, boolean | undefined, boolean>;
  attempts: ColumnType<number, number | undefined, number>;
  created_at: ColumnType<Date, string | undefined, never>;
}
// Database is this slice's Kysely schema view.
export interface Database {
  users: UsersTable;
  companies: CompaniesTable;
  users_profile: UsersProfileTable;
  email_verifications: EmailVerificationsTable;
}

// signupGetUserByEmail :one
export const signupGetUserByEmail = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("users")
    .select([
      "id",
      "email",
      "first_name",
      "last_name",
      "role",
      "user_type",
      "language",
      "is_active",
      "is_verified",
    ])
    .where("email", "=", email)
    .executeTakeFirst()) ?? null;

// signupCreateUser :exec
export const signupCreateUser = (
  db: Kysely<Database>,
  p: {
    id: string;
    email: string;
    firstName: string;
    lastName: string;
    password: string;
    language: string;
    userType: string;
  },
): Promise<unknown> =>
  db
    .insertInto("users")
    .values({
      id: p.id,
      email: p.email,
      first_name: p.firstName,
      last_name: p.lastName,
      password: p.password,
      language: p.language,
      user_type: p.userType,
    })
    .execute();

// signupCreateCompanyProfile :exec — 1:1 company profile (user_id PK,
// 0002_profiles): signup seeds only the NOT NULL name (first + last).
export const signupCreateCompanyProfile = (
  db: Kysely<Database>,
  p: { userId: string; companyName: string },
): Promise<unknown> =>
  db.insertInto("companies").values({ user_id: p.userId, company_name: p.companyName }).execute();

// signupCreateUserProfile :exec — 1:1 personal profile: full_name = first+last.
export const signupCreateUserProfile = (
  db: Kysely<Database>,
  p: { userId: string; fullName: string },
): Promise<unknown> =>
  db.insertInto("users_profile").values({ user_id: p.userId, full_name: p.fullName }).execute();

// signupCreateVerification :exec
export const signupCreateVerification = (
  db: Kysely<Database>,
  p: { id: string; userId: string; token: string; expiresAt: Date },
): Promise<unknown> =>
  db
    .insertInto("email_verifications")
    .values({
      id: p.id,
      user_id: p.userId,
      token: p.token,
      expires_at: p.expiresAt,
    })
    .execute();

// signupLatestVerification :one
export const signupLatestVerification = async (db: Kysely<Database>, userId: string) =>
  (await db
    .selectFrom("email_verifications")
    .select(["id", "user_id", "token", "expires_at", "is_used", "attempts"])
    .where("user_id", "=", userId)
    .orderBy("created_at", "desc")
    .limit(1)
    .executeTakeFirst()) ?? null;

// signupUseVerification :execrows — atomic consume: guards make the code
// single-use even when two requests race it (loser affects 0 rows).
export const signupUseVerification = async (
  db: Kysely<Database>,
  p: { id: string; token: string },
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("email_verifications")
        .set({ is_used: true })
        .where("id", "=", p.id)
        .where("is_used", "=", false)
        .where("token", "=", p.token)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// signupBumpVerification :execrows — failed attempt bookkeeping: after
// maxCodeAttempts wrong guesses the code reads as invalid (brute-force cap).
export const signupBumpVerification = (db: Kysely<Database>, id: string): Promise<unknown> =>
  db
    .updateTable("email_verifications")
    .set({ attempts: sql<number>`attempts + 1` })
    .where("id", "=", id)
    .execute();

// signupSetVerified :exec
export const signupSetVerified = (db: Kysely<Database>, id: string): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ is_verified: true, updated_at: sql`now()` })
    .where("id", "=", id)
    .execute();

// Alias row types (ColumnType wrappers unwrapped = the selected shape).
export type SignupUserSel = Exclude<Awaited<ReturnType<typeof signupGetUserByEmail>>, null>;
export type UsersRow = Selectable<UsersTable>;
export type EmailVerificationsRow = Selectable<EmailVerificationsTable>;
