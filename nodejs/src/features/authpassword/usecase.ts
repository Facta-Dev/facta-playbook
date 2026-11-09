// Features authpassword use case: forgot / validate / reset / change.

import { checkPassword, hashPassword } from "#platform/authx.js";
import { type Clock } from "#shared/clockx.js";
import { newCode, uuidGen } from "#shared/idgen.js";
import crypto from "node:crypto";
import {
  type Reset,
  maxCodeAttempts,
  type User,
  errRateLimited,
  errCodeExpired,
  errCodeInvalid,
  errCodeUsed,
  errPasswordWrong,
  errValidation,
} from "./domain.js";
import type { EmailEnqueuer, Store } from "./ports.js";

// UseCase implements forgot / validate / reset / change.
export class UseCase {
  constructor(
    private readonly store: Store,
    private readonly emails: EmailEnqueuer,
    private readonly clock: Clock,
    private readonly pepper: string,
    private readonly resetTtlMs: number, // code lifetime (30m)
    private readonly windowMs: number, // rate-limit window (5m)
    private readonly maxResets: number, // reset codes allowed per window
    private readonly logOutAllOnReset: boolean,
  ) {}

  // forgot rate-limits, then queues a fresh reset code (no existence leak).
  async forgot(email: string): Promise<void> {
    email = email.toLowerCase().trim();
    const now = this.clock.now();
    const n = await this.store.countResetsSince(email, new Date(now.getTime() - this.windowMs));
    if (n >= this.maxResets) {
      throw errRateLimited;
    }
    const u = await this.store.getUserByEmail(email);
    if (u === null) {
      return;
    }
    const code = newCode(6);
    const r: Reset = {
      id: uuidGen.newId(),
      userId: u.id,
      email,
      token: code,
      expiresAt: new Date(now.getTime() + this.resetTtlMs),
      used: false,
      attempts: 0,
      createdAt: now,
    };
    await this.store.createReset(r);
    await this.enqueue(u, "reset_password", code);
  }

  // validate checks a code without consuming it. Like reset, wrong codes bump
  // the attempt counter; past maxCodeAttempts the code reads as invalid.
  async validate(email: string, code: string): Promise<void> {
    await this.check(email, code);
  }

  // reset consumes the code atomically, sets the new password, auto-verifies
  // the account and (optionally) revokes every active session.
  async reset(email: string, code: string, newPassword: string): Promise<void> {
    email = email.toLowerCase().trim();
    if (newPassword.length < 8) {
      throw errValidation;
    }
    const r = await this.check(email, code);
    const u = await this.store.getUserByEmail(email);
    if (u === null) {
      throw errCodeInvalid;
    }
    const hash = await hashPassword(newPassword, this.pepper);
    const consumed = await this.store.markResetUsed(r.id, r.token);
    if (!consumed) {
      throw errCodeUsed; // a racing request consumed the code first
    }
    await this.store.setPasswordAndVerify(u.id, hash);
    if (this.logOutAllOnReset) {
      await this.store.revokeAllSessions(u.id);
    }
    await this.enqueue(u, "password_changed", "");
  }

  // change verifies the current password, then rotates it and revokes every
  // OTHER active session — the caller's own session survives via jti.
  async change(
    userId: string,
    sessionId: string,
    current: string,
    newPassword: string,
  ): Promise<void> {
    if (newPassword.length < 8) {
      throw errValidation;
    }
    const u = await this.store.getUserByID(userId);
    if (u === null) {
      throw errPasswordWrong;
    }
    try {
      await checkPassword(u.password, current, this.pepper);
    } catch {
      throw errPasswordWrong;
    }
    const hash = await hashPassword(newPassword, this.pepper);
    await this.store.setPassword(u.id, hash);
    await this.store.revokeOtherSessions(u.id, sessionId);
    await this.enqueue(u, "password_changed", "");
  }

  // reset validates code state via check; reset additionally consumes.
  async check(email: string, code: string): Promise<Reset> {
    const r = await this.store.latestReset(email);
    if (r === null) {
      throw errCodeInvalid;
    }
    if (r.used) {
      throw errCodeUsed;
    }
    if (this.clock.now().getTime() > r.expiresAt.getTime()) {
      throw errCodeExpired;
    }
    if (r.attempts >= maxCodeAttempts) {
      throw errCodeInvalid;
    }
    if (!tokenEq(r.token, code)) {
      await this.store.bumpReset(r.id);
      throw errCodeInvalid;
    }
    return r;
  }

  // enqueue renders (per user language) and queues one email job.
  private async enqueue(u: User, template: string, code: string): Promise<void> {
    const locale = u.language === "" ? "en" : u.language;
    await this.emails.enqueue({
      to: u.email,
      templateKey: template as "reset_password" | "password_changed",
      vars: { first_name: u.firstName, code, locale },
    });
  }
}

// tokenEq compares codes in constant time (Buffer length gate first).
const tokenEq = (a: string, b: string): boolean => {
  const ab = Buffer.from(a, "utf8");
  const bb = Buffer.from(b, "utf8");
  return ab.length === bb.length && crypto.timingSafeEqual(ab, bb);
};
