// Features adminusers use case: list / get / update with audit trail plus
// the unauthenticated whitelist bootstrap upgrade.

import { canManage, ErrForbidden, type AuthzContext } from "#shared/authz.js";
import { valid, type Role } from "#shared/rolex.js";
import {
  type Row,
  type Patch,
  type UpgradeResult,
  errNotFound,
  errBadRole,
  errNoChanges,
  errNotWhitelisted,
  userTypeAdmin,
} from "./domain.js";
import type { Store } from "./ports.js";

// UseCase implements list / get / update with audit trail plus the
// unauthenticated whitelist bootstrap upgrade.
export class UseCase {
  private readonly store: Store;
  private readonly whitelist: Set<string>;

  // allowedEmails is the <APP>_ALLOWED_ADMIN_EMAILS whitelist behind the
  // bootstrap upgrade (entries lowercased; empty = endpoint always denies).
  constructor(store: Store, allowedEmails: string[]) {
    this.store = store;
    this.whitelist = new Set(
      allowedEmails.map((e) => e.toLowerCase().trim()).filter((e) => e !== ""),
    );
  }

  // list returns one page plus the total row count.
  async list(limit: number, offset: number): Promise<{ rows: Row[]; total: number }> {
    const rows = await this.store.list(limit, offset);
    const total = await this.store.count();
    return { rows, total };
  }

  // get fetches a single user.
  async get(id: string): Promise<Row> {
    const row = await this.store.get(id);
    if (row === null) {
      throw errNotFound;
    }
    return row;
  }

  // update applies the patch, validates the role, enforces self-or-subordinate
  // (authz: strict rank — an ADMIN row is reachable only by its own, and a
  // stale actor token must still outrank the row), and writes the audit entry.
  async update(
    actorId: string,
    actorRole: Role,
    id: string,
    ip: string,
    ua: string,
    p: Patch,
  ): Promise<Row> {
    if (p.isActive === null && p.isBlocked === null && p.role === "") {
      throw errNoChanges;
    }
    if (p.role !== "" && !valid(p.role as Role)) {
      throw errBadRole;
    }
    // Existence first: an unknown id is 404 and never audited.
    const target = await this.store.get(id);
    if (target === null) {
      throw errNotFound;
    }
    // Self-or-subordinate (shared/authz): an ADMIN cannot manage another
    // ADMIN, and a stale actor role cannot outrank the row's DB role.
    const z: AuthzContext = {
      subjectId: actorId,
      subjectRole: actorRole,
      targetId: target.id,
      targetRole: target.role as Role,
    };
    if (!canManage(z)) {
      throw ErrForbidden;
    }
    await this.store.update(id, p);
    // Audit failure must not block the mutation; it is logged loudly so the
    // trail gap is visible.
    try {
      await this.store.writeAudit(actorId, "user.updated", "user", id, payloadOf(p), ip, ua);
    } catch (err) {
      console.error("audit", { action: "user.updated", userId: id, err });
    }
    return this.get(id);
  }

  // upgradeToAdmin is the unauthenticated bootstrap endpoint (reference
  // upgrade_to_admin parity): a whitelisted email may promote its own account
  // to role+user_type ADMIN. The whitelist is checked before any user lookup,
  // so a non-whitelisted caller cannot probe for account existence; a
  // whitelisted but unknown email is still 404.
  async upgradeToAdmin(rawEmail: string): Promise<UpgradeResult> {
    const email = rawEmail.toLowerCase().trim();
    if (!this.whitelist.has(email)) {
      throw errNotWhitelisted;
    }
    const row = await this.store.getByEmail(email);
    if (row === null) {
      throw errNotFound;
    }
    if (row.role === "ADMIN" && row.userType === userTypeAdmin) {
      return { email, message: "User is already an admin", upgraded: false };
    }
    await this.store.upgradeToAdmin(row.id);
    return {
      email,
      message: `User '${email}' has been upgraded to admin`,
      upgraded: true,
    };
  }
}

// payloadOf renders the update payload for audit_logs (compact JSON).
const payloadOf = (p: Patch): string => {
  const parts: string[] = [];
  if (p.isActive !== null) {
    parts.push(`"is_active":${p.isActive ? "true" : "false"}`);
  }
  if (p.isBlocked !== null) {
    parts.push(`"is_blocked":${p.isBlocked ? "true" : "false"}`);
  }
  if (p.role !== "") {
    parts.push(`"role":"${p.role}"`);
  }
  return `{${parts.join(",")}}`;
};
