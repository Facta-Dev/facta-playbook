// Features authsession ports: consumer-declared interfaces. The Postgres
// adapter (postgres.ts) implements Store; platform/authx satisfies
// TokenMinter.

import type { Identity } from "#platform/authx.js";
import type { UserRow, SessionRow, SessionUser, MultiIPRow, UpdateMeInput } from "./domain.js";

// TokenMinter is the access-token issuance port (platform/authx satisfies it).
export interface TokenMinter {
  mint(id: Identity, ttlMs: number): Promise<{ token: string; exp: Date }>;
}

// Store persists sessions + reads users (impl: postgres.ts). Absent rows
// read as null.
export interface Store {
  getUserByEmail(email: string): Promise<UserRow | null>;
  getUserByID(id: string): Promise<UserRow | null>;
  // updateMe applies the non-null fields (undefined = leave unchanged) and
  // re-reads the profile — update-then-GetMe parity. null when the user does
  // not exist.
  updateMe(userId: string, inp: UpdateMeInput): Promise<UserRow | null>;
  // afterFailedLogin increments SQL-side and returns whether the user just
  // crossed the lock threshold.
  afterFailedLogin(userId: string, lockAt: number): Promise<boolean>;
  afterSuccessfulLogin(userId: string): Promise<void>;
  createSession(s: SessionRow): Promise<void>;
  activeSessionByToken(token: string): Promise<SessionUser | null>;
  // deactivateSession claims the refresh-row outright: false when the
  // session was already deactivated/revoked.
  deactivateSession(sessionId: string): Promise<boolean>;
  // rotateSession rotates the refresh token in place (the row keeps its id
  // and expires_at — fixed absolute lifetime): false when the old token lost
  // the CAS (replay).
  rotateSession(sessionId: string, oldToken: string, s: SessionRow): Promise<boolean>;
  deactivateByTokenForUser(userId: string, token: string): Promise<void>;

  // Session-management views (own + admin) and the anomaly aggregates.
  listSessionsByUser(userId: string, activeOnly: boolean): Promise<SessionRow[]>;
  // currentSession is the user's most recent active session (by last
  // activity); null when none.
  currentSession(userId: string): Promise<SessionRow | null>;
  // revokeSessionForUser claims one active session (CAS on is_active): false
  // when the id is not one of the user's active sessions.
  revokeSessionForUser(userId: string, sessionId: string): Promise<boolean>;
  // revokeAllForUser deactivates every active session of the user and
  // returns how many were claimed.
  revokeAllForUser(userId: string): Promise<number>;
  countExpiredActive(): Promise<number>;
  multiIPSessions(since: Date): Promise<MultiIPRow[]>;
}
