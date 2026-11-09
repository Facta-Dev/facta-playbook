// Features authsignup ports: the consumer-declared interfaces this slice
// needs. The Postgres adapter (postgres.ts) implements Store; platform/email
// satisfies EmailEnqueuer.

import type { Message } from "#platform/email/index.js";
import type { UserRow, Verification } from "./domain.js";

// EmailEnqueuer is this slice's consumer-side port onto the outbox: the slice
// defines the narrow shape it needs; platform/email satisfies it.
export interface EmailEnqueuer {
  enqueue(m: Message): Promise<void>;
}

// Store persists users + verification codes (impl: postgres.ts). Absent rows
// read as null.
export interface Store {
  getUserByEmail(email: string): Promise<UserRow | null>;
  createUser(row: UserRow): Promise<void>;
  createVerification(v: Verification): Promise<void>;
  latestVerification(userId: string): Promise<Verification | null>;
  // useVerification atomically consumes the code; false = raced/taken.
  useVerification(vId: string, token: string): Promise<boolean>;
  // bumpVerification records one wrong-code guess (brute-force cap).
  bumpVerification(vId: string): Promise<void>;
  setEmailVerified(userId: string): Promise<void>;
  // createCompanyProfile seeds the 1:1 companies row (user_id PK).
  createCompanyProfile(userId: string, companyName: string): Promise<void>;
  // createUserProfile seeds the 1:1 users_profile row (user_id PK).
  createUserProfile(userId: string, fullName: string): Promise<void>;
}
