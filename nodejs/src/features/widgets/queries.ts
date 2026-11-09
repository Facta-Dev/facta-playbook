// Features widgets queries — the sqlc analog. The Go template generated row
// types + named queries with sqlc; here Kysely types the schema and builds
// the same five named queries as type-checked function calls (the documented
// codegen delta). Column names are snake_case, exactly as in migrations.

import type { ColumnType, Kysely, Selectable } from "kysely";
import type { WidgetKind } from "./domain.js";

export interface WidgetsTable {
  id: string;
  name: string;
  kind: WidgetKind;
  owner_user_id: string;
  // Set by the database default (now()); never supplied on insert.
  created_at: ColumnType<Date, string | undefined, never>;
}

export interface Database {
  widgets: WidgetsTable;
}

// widgetInsert :exec
export const widgetInsert = (
  db: Kysely<Database>,
  p: { id: string; name: string; kind: WidgetKind; ownerUserId: string },
): Promise<unknown> =>
  db
    .insertInto("widgets")
    .values({
      id: p.id,
      name: p.name,
      kind: p.kind,
      owner_user_id: p.ownerUserId,
    })
    .execute();

// widgetListByOwner :many
export const widgetListByOwner = (
  db: Kysely<Database>,
  p: { ownerId: string; limit: number; offset: number },
): Promise<WidgetsRow[]> =>
  db
    .selectFrom("widgets")
    .selectAll()
    .where("owner_user_id", "=", p.ownerId)
    .orderBy("created_at", "desc")
    .limit(p.limit)
    .offset(p.offset)
    .execute();

// widgetGetByID :one  (null when the row is absent or not the owner's)
export const widgetGetByID = async (
  db: Kysely<Database>,
  p: { id: string; ownerId: string },
): Promise<WidgetsRow | null> =>
  (await db
    .selectFrom("widgets")
    .selectAll()
    .where("id", "=", p.id)
    .where("owner_user_id", "=", p.ownerId)
    .executeTakeFirst()) ?? null;

// widgetUpdate :exec
export const widgetUpdate = (
  db: Kysely<Database>,
  p: { id: string; ownerId: string; name: string; kind: WidgetKind },
): Promise<unknown> =>
  db
    .updateTable("widgets")
    .set({ name: p.name, kind: p.kind })
    .where("id", "=", p.id)
    .where("owner_user_id", "=", p.ownerId)
    .execute();

// widgetDelete :exec
export const widgetDelete = (
  db: Kysely<Database>,
  p: { id: string; ownerId: string },
): Promise<unknown> =>
  db.deleteFrom("widgets").where("id", "=", p.id).where("owner_user_id", "=", p.ownerId).execute();

// Row type as selected: the sqlc-generated WidgetsRow analog (ColumnType
// wrappers unwrapped — created_at reads back as Date).
export type WidgetsRow = Selectable<WidgetsTable>;
