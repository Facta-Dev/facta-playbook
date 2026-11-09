// Features widgets postgres adapts the Kysely database to this slice's Store
// port. The Go template used a sqlc-generated adapter; here Kysely IS the
// typed query layer (same shape, one fewer tool).

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import type { Widget } from "./domain.js";
import type { Store } from "./ports.js";
import {
  widgetDelete,
  widgetGetByID,
  widgetInsert,
  widgetListByOwner,
  widgetUpdate,
  type Database,
  type WidgetsRow,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async insert(w: Widget): Promise<void> {
    await widgetInsert(this.#db, {
      id: w.id,
      name: w.name,
      kind: w.kind,
      ownerUserId: w.ownerUserId,
    });
  }

  async list(ownerId: string, limit: number, offset: number): Promise<Widget[]> {
    const rows = await widgetListByOwner(this.#db, { ownerId, limit, offset });
    return rows.map(conv);
  }

  async get(id: string, ownerId: string): Promise<Widget | null> {
    const r = await widgetGetByID(this.#db, { id, ownerId });
    return r === null ? null : conv(r);
  }

  async update(id: string, ownerId: string, name: string, kind: Widget["kind"]): Promise<void> {
    await widgetUpdate(this.#db, { id, ownerId, name, kind });
  }

  async delete(id: string, ownerId: string): Promise<void> {
    await widgetDelete(this.#db, { id, ownerId });
  }
}

// conv maps a database row onto the slice model.
const conv = (r: WidgetsRow): Widget => ({
  id: r.id,
  name: r.name,
  kind: r.kind,
  ownerUserId: r.owner_user_id,
  createdAt: r.created_at,
});
