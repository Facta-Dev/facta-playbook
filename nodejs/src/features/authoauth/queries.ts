// Features authoauth queries — the sqlc analog (Kysely typed calls). Tables:
// users, sessions. The users view carries the social-flow columns.

import { sql, type ColumnType, type Kysely } from "kysely";
import type { User, SessionRow } from "./domain.js";

// ColumnType<T, I|undefined, U|undefined>: selectable, insert-optional
// (DB default), update-optional.
type Opt<T> = ColumnType<T, T | undefined, T | undefined>;

export interface UsersTable {
  id: string;
  email: string;
  first_name: Opt<string>;
  last_name: Opt<string>;
  password: string;
  role: Opt<string>;
  user_type: Opt<string>;
  language: Opt<string>;
  profile_picture: Opt<string | null>;
  is_active: Opt<boolean>;
  is_blocked: Opt<boolean>;
  is_verified: Opt<boolean>;
  google_id: string | null;
  github_id: string | null;
  updated_at: ColumnType<Date, string | undefined, Date | undefined>;
}

export interface SessionsTable {
  id: string;
  user_id: string;
  refresh_token: string;
  ip_address: Opt<string>;
  user_agent: Opt<string>;
  expires_at: Date;
  is_active: Opt<boolean>;
}

export interface Database {
  users: UsersTable;
  sessions: SessionsTable;
}

// oauthUserSelect is the shared users select list across the lookups.
const oauthUserSelect = [
  "id",
  "email",
  "first_name",
  "last_name",
  "role",
  "user_type",
  "language",
  "is_active",
  "is_blocked",
  "is_verified",
  "profile_picture",
  "google_id",
  "github_id",
] as const;

// oauthGetUserByGoogleID :one
export const oauthGetUserByGoogleID = async (db: Kysely<Database>, googleID: string) =>
  (await db
    .selectFrom("users")
    .select(oauthUserSelect)
    .where("google_id", "=", googleID)
    .executeTakeFirst()) ?? null;

// oauthGetUserByGithubID :one
export const oauthGetUserByGithubID = async (db: Kysely<Database>, githubID: string) =>
  (await db
    .selectFrom("users")
    .select(oauthUserSelect)
    .where("github_id", "=", githubID)
    .executeTakeFirst()) ?? null;

// oauthGetUserByEmail :one
export const oauthGetUserByEmail = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("users")
    .select(oauthUserSelect)
    .where("email", "=", email)
    .executeTakeFirst()) ?? null;

// oauthGetUserByID :one
export const oauthGetUserByID = async (db: Kysely<Database>, id: string) =>
  (await db.selectFrom("users").select(oauthUserSelect).where("id", "=", id).executeTakeFirst()) ??
  null;

// oauthCreateUser :exec — social provisioning: role/language default SQL-side
// ('USER'/'en'), the email arrives provider-verified, and the password is an
// unusable secret.
export const oauthCreateUser = (db: Kysely<Database>, u: User): Promise<unknown> =>
  db
    .insertInto("users")
    .values({
      id: u.id,
      email: u.email,
      first_name: u.firstName,
      last_name: u.lastName,
      password: u.password,
      user_type: u.userType,
      is_verified: true,
      profile_picture: u.picture,
      google_id: u.googleId,
      github_id: u.githubId,
    })
    .execute();

// oauthLinkGoogle :exec — provider proves the email: verify it; fill the
// picture only when empty (COALESCE(NULLIF(...)) keeps existing values).
export const oauthLinkGoogle = (
  db: Kysely<Database>,
  p: { userID: string; providerID: string; picture: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({
      google_id: p.providerID,
      is_verified: true,
      profile_picture: sql<string>`coalesce(nullif(profile_picture, ''), ${p.picture})`,
      updated_at: sql`now()`,
    })
    .where("id", "=", p.userID)
    .execute();

// oauthLinkGithub :exec
export const oauthLinkGithub = (
  db: Kysely<Database>,
  p: { userID: string; providerID: string; picture: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({
      github_id: p.providerID,
      is_verified: true,
      profile_picture: sql<string>`coalesce(nullif(profile_picture, ''), ${p.picture})`,
      updated_at: sql`now()`,
    })
    .where("id", "=", p.userID)
    .execute();

// oauthGithubOnly :exec — link flow: attach the github id only, no
// verification side effects.
export const oauthGithubOnly = (
  db: Kysely<Database>,
  p: { userID: string; githubID: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ github_id: p.githubID, updated_at: sql`now()` })
    .where("id", "=", p.userID)
    .execute();

// oauthSaveGithubLogin :exec — login-flow identity refresh; the noreply
// email upgrade decision is made in the use case and lands here as the new
// email value.
export const oauthSaveGithubLogin = (
  db: Kysely<Database>,
  p: { userID: string; email: string; picture: string },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({
      email: p.email,
      is_verified: true,
      profile_picture: sql<string>`coalesce(nullif(profile_picture, ''), ${p.picture})`,
    })
    .where("id", "=", p.userID)
    .execute();

// oauthCreateSession :exec
export const oauthCreateSession = (db: Kysely<Database>, s: SessionRow): Promise<unknown> =>
  db
    .insertInto("sessions")
    .values({
      id: s.id,
      user_id: s.userId,
      refresh_token: s.refreshToken,
      ip_address: s.ipAddress,
      user_agent: s.userAgent,
      expires_at: s.expiresAt,
    })
    .execute();

// Row-shape alias for the postgres adapter mappers.
export type OauthUserSel = Exclude<Awaited<ReturnType<typeof oauthGetUserByEmail>>, null>;
