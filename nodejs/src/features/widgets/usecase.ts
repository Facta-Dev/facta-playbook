// Features widgets use-case: the CRUD + event publication logic, transport-
// and storage-free. main.ts and the tests instantiate it with the wired
// ports; nil publisher means fire-and-forget off.

import { systemClock, type Clock } from "#shared/clockx.js";
import { uuidGen } from "#shared/idgen.js";
import { roomUser, type RealtimeEvent } from "#platform/realtime/index.js";
import { ErrNotFound, ErrValidation, type Widget, type WidgetKind } from "./domain.js";
import type { Publisher, Store } from "./ports.js";

export class UseCase {
  constructor(
    private readonly store: Store,
    private readonly pub: Publisher | null,
    private readonly clock: Clock = systemClock,
  ) {}

  // create inserts a widget and publishes widget.created to the owner room.
  async create(ownerId: string, name: string, kind: WidgetKind): Promise<Widget> {
    if (name === "") throw ErrValidation;
    const w: Widget = {
      id: uuidGen.newId(),
      name,
      kind,
      ownerUserId: ownerId,
      createdAt: this.clock.now(),
    };
    await this.store.insert(w);
    this.publish(ownerId, "widget.created", w);
    return w;
  }

  // list returns one page of the owner's widgets.
  list(ownerId: string, limit: number, offset: number): Promise<Widget[]> {
    return this.store.list(ownerId, limit, offset);
  }

  // get fetches one of the owner's widgets.
  async get(id: string, ownerId: string): Promise<Widget> {
    const w = await this.store.get(id, ownerId);
    if (w === null) throw ErrNotFound;
    return w;
  }

  // update renames/re-kinds a widget and publishes widget.updated.
  async update(id: string, ownerId: string, name: string, kind: WidgetKind): Promise<Widget> {
    if (name === "") throw ErrValidation;
    if ((await this.store.get(id, ownerId)) === null) throw ErrNotFound;
    await this.store.update(id, ownerId, name, kind);
    const w = await this.store.get(id, ownerId);
    if (w === null) throw ErrNotFound;
    this.publish(ownerId, "widget.updated", w);
    return w;
  }

  // delete removes a widget and publishes widget.deleted.
  async delete(id: string, ownerId: string): Promise<void> {
    const w = await this.store.get(id, ownerId);
    if (w === null) throw ErrNotFound;
    await this.store.delete(id, ownerId);
    this.publish(ownerId, "widget.deleted", w);
  }

  // publish drops the event onto the hub room; delivery is best-effort.
  private publish(ownerId: string, type: string, w: Widget): void {
    if (this.pub === null) return;
    const ev: RealtimeEvent = {
      room: roomUser(ownerId),
      type,
      payload: widgetPayload(w),
      at: new Date(),
    };
    this.pub.publish(ev);
  }
}

// widgetPayload is the JSON payload carried on the wire.
const widgetPayload = (w: Widget): Record<string, string> => ({
  id: w.id,
  name: w.name,
  kind: w.kind,
  owner_user_id: w.ownerUserId,
});
