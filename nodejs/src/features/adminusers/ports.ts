// Features adminusers ports: consumer-declared interfaces; postgres.ts
// implements Store.

import type { Row, Patch } from "./domain.js";

// Store persists admin user reads/updates + the audit trail (impl:
// postgres.ts). Returns read null for "no such row".
export interface Store {
  list(limit: number, offset: number): Promise<Row[]>;
  count(): Promise<number>;
  get(id: string): Promise<Row | null>;
  getByEmail(email: string): Promise<Row | null>;
  update(id: string, p: Patch): Promise<void>;
  // upgradeToAdmin sets role + user_type to ADMIN on the user row.
  upgradeToAdmin(id: string): Promise<void>;
  // writeAudit records one audit_logs entry (actor may be unknown).
  writeAudit(
    actorID: string,
    action: string,
    entity: string,
    entityID: string,
    payload: string,
    ip: string,
    ua: string,
  ): Promise<void>;
}
