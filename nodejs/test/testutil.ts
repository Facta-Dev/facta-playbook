// Shared test doubles: a clock the harness controls, a recorded email
// enqueuer, the real HS256 minter/verifier pair (acceptance mints live
// tokens, it does not fake cryptography) and a deterministic, UUID-shaped
// id sequence.

import { newHS256Verifier, newHS256Minter } from "#platform/authx.js";
import type { Verifier, Minter } from "#platform/authx.js";
import type { Clock } from "#shared/clockx.js";
import type { Message } from "#platform/email/index.js";

// Shared test credentials; TestIssuer matches bin/api.ts's issuer so the
// same rename substitution keeps verifier and minter aligned.
export const TestJWTSecret = "test-secret-32-bytes-0123456789abcedf";
export const TestPepper = "test-pepper-32-bytes-0123456789abcdef";
export const TestIssuer = "<app>-api";

// newMinter/newVerifier build the real HS256 pair every test suite shares.
export const newMinter = (): Minter => newHS256Minter(TestJWTSecret, TestIssuer);
export const newVerifier = (): Verifier => newHS256Verifier(TestJWTSecret, TestIssuer);

// TestClock is the harness-controlled clock: "N hours pass" steps advance it
// so TTL expiry is deterministic.
export class TestClock implements Clock {
  #now: Date;

  constructor(start: Date = new Date(1_700_000_000 * 1000)) {
    this.#now = new Date(start.getTime());
  }

  now(): Date {
    return new Date(this.#now.getTime());
  }

  // advance moves the clock forward by n units of the named kind.
  advance(n: number, unit: "min" | "h"): void {
    const ms = unit === "h" ? n * 3_600_000 : n * 60_000;
    this.#now = new Date(this.#now.getTime() + ms);
  }
}

// RecordedEmails is the fake EmailEnqueuer/Queue: it records every message
// so feature steps can count enqueues per template/recipient.
export class RecordedEmails {
  sent: Message[] = [];

  async enqueue(m: Message): Promise<void> {
    this.sent.push(m);
  }

  // countOf counts the enqueues for one recipient/template pair.
  countOf(template: string, to: string): number {
    return this.sent.filter((m) => m.templateKey === template && m.to === to).length;
  }

  // latestCode returns the 6-digit code of the newest matching message.
  latestCode(to: string, templateKey: string): string | null {
    for (let i = this.sent.length - 1; i >= 0; i--) {
      const m = this.sent[i]!;
      if (m.to === to && m.templateKey === templateKey) {
        return typeof m.vars?.code === "string" ? m.vars.code : null;
      }
    }
    return null;
  }

  clear(): void {
    this.sent = [];
  }
}

// Deterministic UUID-shaped ids: the fakes derive entity ids the same way,
// so assertions on stored rows stay readable.
let idSeq = 0;
export const resetIDs = (): void => {
  idSeq = 0;
};
export const nextID = (): string => {
  idSeq += 1;
  return `00000000-0000-0000-0000-${String(idSeq).padStart(12, "0")}`;
};
