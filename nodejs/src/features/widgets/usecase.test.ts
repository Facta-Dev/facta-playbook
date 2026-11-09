// The use-case is transport- and storage-free, so these tests fake the store
// and use the real hub — mirroring the Go template's usecase_test.go.

import { describe, expect, it } from "vitest";
import { fixedClock } from "#shared/clockx.js";
import { newHub, roomUser } from "#platform/realtime/index.js";
import type { Widget } from "./domain.js";
import type { Store } from "./ports.js";
import { UseCase } from "./usecase.js";

// FakeStore keeps a flat map; every lookup re-checks the owner.
class FakeStore implements Store {
  rows = new Map<string, Widget>();

  async insert(w: Widget): Promise<void> {
    this.rows.set(w.id, w);
  }

  async list(ownerId: string, limit: number, offset: number): Promise<Widget[]> {
    const owned = [...this.rows.values()].filter((w) => w.ownerUserId === ownerId);
    return owned.slice(offset, offset + limit);
  }

  async get(id: string, ownerId: string): Promise<Widget | null> {
    const w = this.rows.get(id) ?? null;
    return w !== null && w.ownerUserId === ownerId ? w : null;
  }

  async update(id: string, ownerId: string, name: string, kind: string): Promise<void> {
    const w = this.rows.get(id);
    if (w !== undefined && w.ownerUserId === ownerId) {
      this.rows.set(id, { ...w, name, kind: kind as Widget["kind"] });
    }
  }

  async delete(id: string, ownerId: string): Promise<void> {
    const w = this.rows.get(id);
    if (w !== undefined && w.ownerUserId === ownerId) this.rows.delete(id);
  }
}

const clock = fixedClock(new Date(1700 * 1000));

describe("widgets use-case", () => {
  it("CRUD roundtrips and publishes", async () => {
    const store = new FakeStore();
    const uc = new UseCase(store, newHub(4), clock);

    const w = await uc.create("owner-1", "gearbox", "cog");
    expect(w.id).not.toBe("");
    expect(w.createdAt.toISOString()).toBe(new Date(1700 * 1000).toISOString());

    const upd = await uc.update(w.id, "owner-1", "gearbox-v2", "flange");
    expect(upd.name).toBe("gearbox-v2");

    await uc.delete(w.id, "owner-1");
    await expect(uc.get(w.id, "owner-1")).rejects.toMatchObject({
      code: "NOT_FOUND",
      http: 404,
    });
  });

  it("events are room-scoped", async () => {
    const store = new FakeStore();
    const hub = newHub(4);
    const uc = new UseCase(store, hub, clock);

    // Subscribe the owner room before the mutation.
    const sub = hub.subscribe(roomUser("owner-1"));

    await uc.create("owner-1", "gearbox", "cog");
    const ev = await sub.next();
    expect(ev.done).toBe(false);
    expect(ev.value?.type).toBe("widget.created");
    expect(ev.value?.room).toBe(roomUser("owner-1"));

    // The other owner's room stays silent: publish then drain-peek.
    await uc.create("owner-2", "other", "cog");
    const none = await Promise.race([
      sub.next().then(() => "leaked"),
      new Promise<"silent">((r) => setTimeout(() => r("silent"), 50)),
    ]);
    expect(none).toBe("silent");
    sub.cancel();
  });

  it("list is owner-scoped", async () => {
    const store = new FakeStore();
    const uc = new UseCase(store, null, clock);
    await uc.create("owner-1", "a", "cog");
    await uc.create("owner-1", "b", "cog");
    await uc.create("owner-2", "c", "cog");

    const rows = await uc.list("owner-1", 10, 0);
    expect(rows).toHaveLength(2);
    for (const r of rows) expect(r.ownerUserId).toBe("owner-1");
  });

  it("validates empty names and missing rows", async () => {
    const store = new FakeStore();
    const uc = new UseCase(store, null, clock);
    await expect(uc.create("owner-1", "", "cog")).rejects.toMatchObject({
      code: "VALIDATION_ERROR",
    });
    await expect(uc.delete("ghost", "owner-1")).rejects.toMatchObject({ code: "NOT_FOUND" });
    await expect(uc.get("ghost", "owner-1")).rejects.toMatchObject({ code: "NOT_FOUND" });
  });
});
