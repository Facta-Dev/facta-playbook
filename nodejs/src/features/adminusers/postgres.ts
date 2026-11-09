// Features adminusers postgres adapts the Kysely database to this slice's
// Store port (the sqlc adapter's analog). Rows map 1:1 (NULL names read as
// their Opt column type; the audit row ids are generated here).

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import { uuidGen } from "#shared/idgen.js";
import type { Row, Patch } from "./domain.js";
import type { Store } from "./ports.js";
import {
  adminListUsers,
  adminCountUsers,
  adminGetUser,
  adminGetUserByEmail,
  adminUpdateUser,
  adminUpgradeToAdmin,
  adminWriteAudit,
  type Database,
  type AdminSel,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async list(limit: number, offset: number): Promise<Row[]> {
    const rows = await adminListUsers(this.#db, { limit, offset });
    return rows.map(rowToList);
  }

  async count(): Promise<number> {
    return adminCountUsers(this.#db);
  }

  async get(id: string): Promise<Row | null> {
    const r = await adminGetUser(this.#db, id);
    return r === null ? null : rowToRow(r);
  }

  // getByEmail mirrors get on the unique email column.
  async getByEmail(email: string): Promise<Row | null> {
    const r = await adminGetUserByEmail(this.#db, email);
    return r === null ? null : rowToRow(r);
  }

  async update(id: string, patch: Patch): Promise<void> {
    await adminUpdateUser(this.#db, { id, patch });
  }

  // upgradeToAdmin moves role + user_type to ADMIN (bootstrap whitelist route).
  async upgradeToAdmin(id: string): Promise<void> {
    await adminUpgradeToAdmin(this.#db, id);
  }

  async writeAudit(
    actorID: string,
    action: string,
    entity: string,
    entityID: string,
    payload: string,
    ip: string,
    ua: string,
  ): Promise<void> {
    await adminWriteAudit(this.#db, {
      id: uuidGen.newId(),
      actorId: actorID,
      action,
      entity,
      entityId: entityID,
      payload,
      ip,
      ua,
    });
  }
}

// coreRow maps the shared admin-view columns (everything but last_login).
const coreRow = (r: Omit<AdminSel, "last_login">): Omit<Row, "lastLogin"> => ({
  id: r.id,
  email: r.email,
  firstName: r.first_name,
  lastName: r.last_name,
  role: r.role,
  userType: r.user_type,
  isActive: r.is_active,
  isBlocked: r.is_blocked,
  isVerified: r.is_verified,
  language: r.language,
  createdAt: r.created_at,
});

// rowToRow maps the full admin view; a NULL last_login reads as its zero
// value (pgtype parity).
const rowToRow = (r: AdminSel): Row => ({ ...coreRow(r), lastLogin: r.last_login ?? new Date(0) });

// rowToList maps the page view: it never selects last_login (zero-value
// parity), which the wire mapper drops.
const rowToList = (r: Omit<AdminSel, "last_login">): Row => ({
  ...coreRow(r),
  lastLogin: new Date(0),
});
