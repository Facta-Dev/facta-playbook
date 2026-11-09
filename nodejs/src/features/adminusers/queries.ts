// Features adminusers queries — the sqlc analog. Kysely types the schema
// and builds the same named queries. Tables: users, audit_logs.

import { sql, type ColumnType, type Kysely } from "kysely";
import type { Patch } from "./domain.js";

// ColumnType<T, I|undefined, U|undefined>: selectable, insert-optional
// (DB default), update-optional.
type Opt<T> = ColumnType<T, T | undefined, T | undefined>;

export interface UsersTable {
  id: string;
  email: string;
  first_name: Opt<string>;
  last_name: Opt<string>;
  role: Opt<string>;
  user_type: Opt<string>;
  language: Opt<string>;
  is_active: Opt<boolean>;
  is_blocked: Opt<boolean>;
  is_verified: Opt<boolean>;
  last_login: ColumnType<Date | null, Date | null, Date | null>;
  created_at: ColumnType<Date, string | undefined, never>;
  updated_at: ColumnType<Date, string | undefined, Date | undefined>;
}

export interface AuditLogsTable {
  id: string;
  actor_user_id: ColumnType<string | null, string | null | undefined, string | null | undefined>;
  action: string;
  entity: Opt<string>;
  entity_id: ColumnType<string | null, string | null | undefined, string | null | undefined>;
  payload: Opt<string>;
  ip_address: ColumnType<string | null, string | null | undefined, string | null | undefined>;
  user_agent: ColumnType<string | null, string | null | undefined, string | null | undefined>;
  created_at: ColumnType<Date, string | undefined, never>;
}

// Database is this slice's Kysely schema view.
export interface Database {
  users: UsersTable;
  audit_logs: AuditLogsTable;
}

// adminListUsers :many — newest signups first.
export const adminListUsers = (db: Kysely<Database>, p: { limit: number; offset: number }) =>
  db
    .selectFrom("users")
    .select([
      "id",
      "email",
      "first_name",
      "last_name",
      "role",
      "user_type",
      "is_active",
      "is_blocked",
      "is_verified",
      "language",
      "created_at",
    ])
    .orderBy("created_at", "desc")
    .limit(p.limit)
    .offset(p.offset)
    .execute();

// adminCountUsers :one
export const adminCountUsers = async (db: Kysely<Database>): Promise<number> =>
  (
    await db
      .selectFrom("users")
      .select((eb) => eb.fn.count<number>("id").as("n"))
      .executeTakeFirst()
  )?.n ?? 0;

// adminGetUser :one — the admin view incl. last_login.
export const adminGetUser = async (db: Kysely<Database>, id: string) =>
  (await db
    .selectFrom("users")
    .select([
      "id",
      "email",
      "first_name",
      "last_name",
      "role",
      "user_type",
      "is_active",
      "is_blocked",
      "is_verified",
      "language",
      "created_at",
      "last_login",
    ])
    .where("id", "=", id)
    .executeTakeFirst()) ?? null;

// adminGetUserByEmail :one
export const adminGetUserByEmail = async (db: Kysely<Database>, email: string) =>
  (await db
    .selectFrom("users")
    .select([
      "id",
      "email",
      "first_name",
      "last_name",
      "role",
      "user_type",
      "is_active",
      "is_blocked",
      "is_verified",
      "language",
      "created_at",
      "last_login",
    ])
    .where("email", "=", email)
    .executeTakeFirst()) ?? null;

// AdminSel aliases the shared admin-view row shape (both lookups above).
export type AdminSel = Exclude<Awaited<ReturnType<typeof adminGetUser>>, null>;

// adminUpgradeToAdmin :exec — bootstrap whitelist upgrade (reference
// upgrade_to_admin parity): both the role and the user_type move to ADMIN so
// RBAC and the profile rules agree.
export const adminUpgradeToAdmin = (db: Kysely<Database>, id: string): Promise<unknown> =>
  db
    .updateTable("users")
    .set({ role: "ADMIN", user_type: "ADMIN", updated_at: sql`now()` })
    .where("id", "=", id)
    .execute();

// adminUpdateUser :exec — partial update: the COALESCE-narg SQL is a
// conditional spread (absent keys are omitted, leaving the column untouched).
export const adminUpdateUser = (
  db: Kysely<Database>,
  p: { id: string; patch: Patch },
): Promise<unknown> =>
  db
    .updateTable("users")
    .set({
      ...(p.patch.isActive !== null && { is_active: p.patch.isActive }),
      ...(p.patch.isBlocked !== null && { is_blocked: p.patch.isBlocked }),
      ...(p.patch.role !== "" && { role: p.patch.role }),
      updated_at: sql`now()`,
    })
    .where("id", "=", p.id)
    .execute();

// adminWriteAudit :exec — one audit_logs entry (actor/entity_id/ip/ua may be
// unknown: NULL then).
export const adminWriteAudit = (
  db: Kysely<Database>,
  e: {
    id: string;
    actorId: string;
    action: string;
    entity: string;
    entityId: string;
    payload: string;
    ip: string;
    ua: string;
  },
): Promise<unknown> =>
  db
    .insertInto("audit_logs")
    .values({
      id: e.id,
      actor_user_id: e.actorId === "" ? null : e.actorId,
      action: e.action,
      entity: e.entity,
      entity_id: e.entityId === "" ? null : e.entityId,
      payload: e.payload,
      ip_address: e.ip === "" ? null : e.ip,
      user_agent: e.ua === "" ? null : e.ua,
    })
    .execute();
