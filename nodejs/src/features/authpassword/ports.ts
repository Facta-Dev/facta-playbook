// Features authpassword ports: consumer-declared interfaces. The Postgres
// adapter (postgres.ts) implements Store; the email outbox platform satisfies
// EmailEnqueuer.

import type { Message } from "#platform/email/index.js";
import type { Reset, User } from "./domain.js";

// EmailEnqueuer is this slice's consumer-side port onto the outbox.
export interface EmailEnqueuer {
  enqueue(m: Message): Promise<void>;
}

// Store persists reset codes + password changes (impl: postgres.ts).
export interface Store {
  countResetsSince(email: string, since: Date): Promise<number>;
  getUserByEmail(email: string): Promise<User | null>;
  getUserByID(id: string): Promise<User | null>;
  createReset(r: Reset): Promise<void>;
  latestReset(email: string): Promise<Reset | null>;
  // markResetUsed atomically consumes the code; false = raced/taken.
  markResetUsed(id: string, token: string): Promise<boolean>;
  // bumpReset records one wrong-code guess (brute-force cap).
  bumpReset(id: string): Promise<void>;
  setPasswordAndVerify(id: string, hash: string): Promise<void>;
  setPassword(id: string, hash: string): Promise<void>;
  revokeAllSessions(userId: string): Promise<void>;
  // revokeOtherSessions keeps the caller's session (jti == sessions.id).
  revokeOtherSessions(userId: string, sessionId: string): Promise<void>;
}
