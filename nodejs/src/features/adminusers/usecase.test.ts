// The use-case is transport- and storage-free, so these tests fake the
// store — mirroring the Go template's usecase_test.go / fakes_test.go.

import { describe, expect, it } from "vitest";
import { Admin, type Role } from "#shared/rolex.js";
import type { Row, Patch } from "./domain.js";
import { userTypeAdmin } from "./domain.js";
import type { Store } from "./ports.js";
import { UseCase } from "./usecase.js";

// FakeStore keeps a flat map; update mutates in place, audit records the
// write-audit calls.
class FakeStore implements Store {
  rows = new Map<string, Row>();
  audit: string[] = [];

  constructor(rows: Row[]) {
    for (const r of rows) {
      this.rows.set(r.id, r);
    }
  }

  async list(): Promise<Row[]> {
    return [...this.rows.values()];
  }
  async count(): Promise<number> {
    return this.rows.size;
  }
  async get(id: string): Promise<Row | null> {
    return this.rows.get(id) ?? null;
  }
  async getByEmail(email: string): Promise<Row | null> {
    for (const r of this.rows.values()) {
      if (r.email.toLowerCase() === email.toLowerCase()) {
        return r;
      }
    }
    return null;
  }
  // update: DB parity — a missing id affects 0 rows, no error.
  async update(id: string, p: Patch): Promise<void> {
    const u = this.rows.get(id);
    if (u === undefined) {
      return;
    }
    if (p.isActive !== null) {
      u.isActive = p.isActive;
    }
    if (p.isBlocked !== null) {
      u.isBlocked = p.isBlocked;
    }
    if (p.role !== "") {
      u.role = p.role;
    }
  }
  async upgradeToAdmin(id: string): Promise<void> {
    const u = this.rows.get(id);
    if (u !== undefined) {
      u.role = Admin;
      u.userType = userTypeAdmin;
    }
  }
  async writeAudit(
    actorId: string,
    action: string,
    entity: string,
    entityId: string,
    payload: string,
  ): Promise<void> {
    this.audit.push(`${action}|${entity}|${entityId}|${payload}|${actorId}`);
  }
}

// newAdminRow seeds one fixed-time user row.
const newAdminRow = (id: string): Row => ({
  id,
  email: `u${id}@ex.com`,
  firstName: "Ada",
  lastName: "L",
  role: "USER" as Role,
  userType: "USER",
  isActive: true,
  isBlocked: false,
  isVerified: true,
  language: "en",
  createdAt: new Date(1700 * 1000),
  lastLogin: new Date(0),
});

describe("adminusers use case", () => {
  it("list returns the page plus the total; missing user 404s", async () => {
    const store = new FakeStore([newAdminRow("u-1")]);
    const uc = new UseCase(store, []);

    const { rows, total } = await uc.list(20, 0);
    expect(total).toBe(1);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.id).toBe("u-1");

    const got = await store.get("u-1");
    expect(got?.email).toBe("uu-1@ex.com");
    await expect(uc.get("ghost")).rejects.toMatchObject({ code: "NOT_FOUND" });
  });

  it("update validates the role, the rank and the audit trail", async () => {
    const store = new FakeStore([newAdminRow("u-1")]);
    const uc = new UseCase(store, []);

    await expect(
      uc.update("admin-1", "ADMIN", "u-1", "9.9.9.9", "ua", patch({ role: "WIZARD" })),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
    await expect(
      uc.update("admin-1", "ADMIN", "u-1", "9.9.9.9", "ua", patch({})),
    ).rejects.toMatchObject({ code: "VALIDATION_ERROR" });
    await expect(
      uc.update("admin-1", "ADMIN", "ghost", "9.9.9.9", "ua", patch({ role: "MODERATOR" })),
    ).rejects.toMatchObject({ code: "NOT_FOUND" });
    // Strict rank: an admin cannot reach another admin's row.
    const peer = newAdminRow("u-2");
    peer.role = "ADMIN";
    store.rows.set("u-2", peer);
    await expect(
      uc.update("admin-1", "ADMIN", "u-2", "9.9.9.9", "ua", patch({ role: "MODERATOR" })),
    ).rejects.toMatchObject({ code: "AUTH_FORBIDDEN" });

    // The target is a USER row, so the admin manages it, and the audit lands.
    const row = await uc.update(
      "admin-1",
      "ADMIN",
      "u-1",
      "9.9.9.9",
      "ua",
      patch({ role: "MODERATOR" }),
    );
    expect(row.role).toBe("MODERATOR");
    expect(store.audit).toEqual(['user.updated|user|u-1|{"role":"MODERATOR"}|admin-1']);
  });

  it("update applies nullable booleans and audits them", async () => {
    const store = new FakeStore([newAdminRow("u-1")]);
    const uc = new UseCase(store, []);

    const row = await uc.update("admin-1", "ADMIN", "u-1", "", "", patch({ isActive: false }));
    expect(row.isActive).toBe(false);
    expect((await store.get("u-1"))?.isActive).toBe(false);
    expect(store.audit).toEqual(['user.updated|user|u-1|{"is_active":false}|admin-1']);
  });

  it("upgradeToAdmin gates on the whitelist before any lookup", async () => {
    const store = new FakeStore([newAdminRow("u-1")]);
    const uc = new UseCase(store, ["UU-1@ex.com"]);

    await expect(uc.upgradeToAdmin("ghost@ex.com")).rejects.toMatchObject({
      code: "AUTH_FORBIDDEN",
    });
    const res = await uc.upgradeToAdmin("uu-1@ex.com");
    expect(res).toEqual({
      email: "uu-1@ex.com",
      message: "User 'uu-1@ex.com' has been upgraded to admin",
      upgraded: true,
    });
    // Already an admin: upgraded=false, no second write.
    const again = await uc.upgradeToAdmin("uu-1@ex.com");
    expect(again.upgraded).toBe(false);
    expect(again.message).toBe("User is already an admin");
  });
});

// patch builds a Patch literal (nulls = "leave untouched").
const patch = (p: Partial<Patch>): Patch => ({ isActive: null, isBlocked: null, role: "", ...p });
