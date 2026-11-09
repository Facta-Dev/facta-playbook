// Features authsession queries — the sqlc analog. Kysely types the schema
// and builds the same named queries (documented codegen delta). Column
// names snake_case, exactly as in migrations.

import { sql, type ColumnType, type Kysely } from "kysely";
import type { UpdateMeInput } from "./domain.js";

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
  phone_number: Opt<string>;
  profile_picture: Opt<string>;
  subscription: Opt<string>;
  address: Opt<string>;
  postal_code: Opt<string>;
  is_active: Opt<boolean>;
  is_blocked: Opt<boolean>;
  is_verified: Opt<boolean>;
  retry_count: Opt<number>;
  google_id: string | null;
  github_id: string | null;
  last_login: ColumnType<Date | null, Date | null, Date | null>;
  created_at: ColumnType<Date, string | undefined, never>;
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
  created_at: ColumnType<Date, string | undefined, Date | undefined>;
  last_activity: ColumnType<Date, string | undefined, Date | undefined>;
}

// Database is this slice's Kysely schema view.
export interface Database {
  users: UsersTable;
  sessions: SessionsTable;
}

// sessionLoginGetUserByEmail :one
export const sessionLoginGetUserByEmail = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("users")
    .select([
      "id",
      "email",
      "password",
      "role",
      "language",
      "is_active",
      "is_blocked",
      "is_verified",
      "retry_count",
      "google_id",
      "github_id",
    ])
    .where("email", "=", email)
    .executeTakeFirst()) ?? null;

// sessionUserByID :one — the me-row shape (profile + audit columns).
export const sessionUserByID = async (db: Kysely<Database>, id: string) =>
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
      "is_blocked",
      "is_verified",
      "retry_count",
      "phone_number",
      "profile_picture",
      "subscription",
      "address",
      "postal_code",
      sql<boolean>`google_id is not null`.as("google_linked"),
      sql<boolean>`github_id is not null`.as("github_linked"),
      "last_login",
      "created_at",
      "updated_at",
    ])
    .where("id", "=", id)
    .executeTakeFirst()) ?? null;

// sessionAfterFailedLogin :one — the counter increments SQL-side so
// concurrent failures cannot lose an increment; the DB computes the lock.
export const sessionAfterFailedLogin = async (
  db: Kysely<Database>,
  p: { id: string; lockAt: number },
): Promise<boolean> =>
  (
    await db
      .updateTable("users")
      .set({
        retry_count: sql<number>`retry_count + 1`,
        is_blocked: sql<boolean>`retry_count + 1 >= ${p.lockAt}`,
        updated_at: sql`now()`,
      })
      .where("id", "=", p.id)
      .returning("is_blocked")
      .executeTakeFirst()
  )?.is_blocked ?? false;

// sessionAfterSuccessfulLogin :exec
export const sessionAfterSuccessfulLogin = (db: Kysely<Database>, id: string): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ retry_count: 0, last_login: sql`now()`, updated_at: sql`now()` })
    .where("id", "=", id)
    .execute();

// sessionCreate :exec
export const sessionCreate = (
  db: Kysely<Database>,
  s: {
    id: string;
    userId: string;
    refreshToken: string;
    ipAddress: string;
    userAgent: string;
    expiresAt: Date;
  },
): Promise<unknown> =>
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

// sessionGetActiveByToken :one — the joined account state rides along so
// refresh can re-check it.
export const sessionGetActiveByToken = async (db: Kysely<Database>, token: string) =>
  (await db
    .selectFrom("sessions as s")
    .innerJoin("users as u", "u.id", "s.user_id")
    .select([
      "s.id as session_id",
      "s.user_id",
      "s.refresh_token",
      "s.expires_at",
      "s.is_active",
      "u.email",
      "u.role",
      "u.is_active as user_is_active",
      "u.is_blocked",
      "u.is_verified",
    ])
    .where("s.refresh_token", "=", token)
    .where("s.is_active", "=", true)
    .where("s.expires_at", ">", sql<Date>`now()`)
    .executeTakeFirst()) ?? null;

// sessionDeactivate :execrows — atomic claim of the refresh-row.
export const sessionDeactivate = async (db: Kysely<Database>, id: string): Promise<number> =>
  Number(
    (
      await db
        .updateTable("sessions")
        .set({ is_active: false })
        .where("id", "=", id)
        .where("is_active", "=", true)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// sessionRotateTokens :execrows — in-place rotation: the row keeps its id
// and expires_at (fixed absolute lifetime, never slides forward); only the
// token, ip/ua and last_activity change. The WHERE guard on the old token is
// the compare-and-swap — a replayed token can only ever win once.
export const sessionRotateTokens = async (
  db: Kysely<Database>,
  p: { refreshToken: string; ipAddress: string; userAgent: string; id: string; oldToken: string },
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("sessions")
        .set({
          refresh_token: p.refreshToken,
          ip_address: p.ipAddress,
          user_agent: p.userAgent,
          last_activity: sql`now()`,
        })
        .where("id", "=", p.id)
        .where("refresh_token", "=", p.oldToken)
        .where("is_active", "=", true)
        .where("expires_at", ">", sql<Date>`now()`)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// sessionDeactivateByTokenForUser :exec
export const sessionDeactivateByTokenForUser = (
  db: Kysely<Database>,
  p: { refreshToken: string; userId: string },
): Promise<unknown> =>
  db
    .updateTable("sessions")
    .set({ is_active: false })
    .where("refresh_token", "=", p.refreshToken)
    .where("user_id", "=", p.userId)
    .where("is_active", "=", true)
    .execute();

// sessionListByUser :many — newest activity first; active-only filter is the
// Go `(active_only = false OR is_active)` folded into a conditional where.
export const sessionListByUser = (
  db: Kysely<Database>,
  p: { userId: string; activeOnly: boolean },
) =>
  db
    .selectFrom("sessions")
    .select([
      "id",
      "user_id",
      "refresh_token",
      "ip_address",
      "user_agent",
      "is_active",
      "last_activity",
      "created_at",
      "expires_at",
    ])
    .where("user_id", "=", p.userId)
    .$if(p.activeOnly, (qb) => qb.where("is_active", "=", true))
    .orderBy("last_activity", "desc")
    .execute();

// sessionCurrentByUser :one — the user's most recent active session.
export const sessionCurrentByUser = async (db: Kysely<Database>, userId: string) =>
  (await db
    .selectFrom("sessions")
    .select([
      "id",
      "user_id",
      "refresh_token",
      "ip_address",
      "user_agent",
      "is_active",
      "last_activity",
      "created_at",
      "expires_at",
    ])
    .where("user_id", "=", userId)
    .where("is_active", "=", true)
    .orderBy("last_activity", "desc")
    .limit(1)
    .executeTakeFirst()) ?? null;

// SessionSel aliases the shared select-list shape of the two views above.
export type SessionSel = Exclude<Awaited<ReturnType<typeof sessionCurrentByUser>>, null>;

// sessionRevokeForUser :execrows — claim one active session of the user.
export const sessionRevokeForUser = async (
  db: Kysely<Database>,
  p: { userId: string; sessionId: string },
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("sessions")
        .set({ is_active: false })
        .where("user_id", "=", p.userId)
        .where("id", "=", p.sessionId)
        .where("is_active", "=", true)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// sessionRevokeAllForUser :execrows — how many active sessions were claimed.
export const sessionRevokeAllForUser = async (
  db: Kysely<Database>,
  userId: string,
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("sessions")
        .set({ is_active: false })
        .where("user_id", "=", userId)
        .where("is_active", "=", true)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// sessionCountExpiredActive :one — expired-but-still-active rows (janitor
// would reap them; the anomaly report surfaces the count before that).
export const sessionCountExpiredActive = async (db: Kysely<Database>): Promise<number> =>
  (
    await db
      .selectFrom("sessions")
      .select((eb) => eb.fn.count<number>("id").as("count"))
      .where("is_active", "=", true)
      .where("expires_at", "<", sql<Date>`now()`)
      .executeTakeFirst()
  )?.count ?? 0;

// sessionAnomalyMultiIP :many — users whose active sessions since the window
// start span more distinct IPs than the SQL threshold (> 3).
export const sessionAnomalyMultiIP = async (
  db: Kysely<Database>,
  since: Date,
): Promise<Array<{ user_id: string; ip_count: number }>> =>
  (await db
    .selectFrom("sessions")
    .select(["user_id", sql<number>`count(distinct ip_address)::int`.as("ip_count")])
    .where("is_active", "=", true)
    .where("last_activity", ">=", since)
    .groupBy("user_id")
    .having(sql<boolean>`count(distinct ip_address)::int > 3`)
    .execute()) as Array<{ user_id: string; ip_count: number }>;

// sessionUpdateMe :execrows — partial update: absent fields are omitted
// (leave unchanged), "" sets the empty string; never null (geo columns are
// dropped — no countries/cities tables). Kysely omits absent keys, so the
// Go COALESCE-narg SQL is a conditional spread.
export const sessionUpdateMe = async (
  db: Kysely<Database>,
  p: { userId: string; inp: UpdateMeInput },
): Promise<number> =>
  Number(
    (
      await db
        .updateTable("users")
        .set({
          ...(p.inp.firstName !== undefined && { first_name: p.inp.firstName }),
          ...(p.inp.lastName !== undefined && { last_name: p.inp.lastName }),
          ...(p.inp.phone !== undefined && { phone_number: p.inp.phone }),
          ...(p.inp.picture !== undefined && { profile_picture: p.inp.picture }),
          ...(p.inp.language !== undefined && { language: p.inp.language }),
          ...(p.inp.address !== undefined && { address: p.inp.address }),
          ...(p.inp.postal !== undefined && { postal_code: p.inp.postal }),
          updated_at: sql`now()`,
        })
        .where("id", "=", p.userId)
        .executeTakeFirst()
    )?.numUpdatedRows ?? 0n,
  );

// Row-shape aliases for the postgres adapter mappers.
export type SessionLoginRow = Exclude<Awaited<ReturnType<typeof sessionLoginGetUserByEmail>>, null>;
export type SessionMeRow = Exclude<Awaited<ReturnType<typeof sessionUserByID>>, null>;
export type SessionActiveRow = Exclude<Awaited<ReturnType<typeof sessionGetActiveByToken>>, null>;
