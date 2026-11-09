// Features authsignup use case: signup / verify / resend flows. Port of the
// Go template's authsignup/usecase.go.

import crypto from "node:crypto";
import { hashPassword } from "#platform/authx.js";
import { type Clock } from "#shared/clockx.js";
import { newCode, uuidGen } from "#shared/idgen.js";
import {
  maxCodeAttempts,
  normalizeUserType,
  errCodeExpired,
  errCodeInvalid,
  errCodeUsed,
  errValidation,
  errEmailTakenActive,
  UserTypeUser,
  UserTypeCompany,
  type UserRow,
  type Verification,
} from "./domain.js";
import type { EmailEnqueuer, Store } from "./ports.js";

// SignupInput carries the decoded request. userType is optional (ADMIN,
// COMPANY, USER — case-insensitive); blank resolves to COMPANY.
export interface SignupInput {
  email: string;
  firstName: string;
  lastName: string;
  password: string;
  language: string;
  userType: string;
}

// UseCase implements the signup / verify / resend flows. Port of usecase.go.
export class UseCase {
  constructor(
    // ponytail: fields not readonly — nothing mutates them, `readonly` was noise.
    private readonly store: Store,
    private readonly emails: EmailEnqueuer,
    private readonly clock: Clock,
    private readonly pepper: string,
    private readonly verifyTtlMs: number,
  ) {}

  // signup normalizes the email, creates the user unverified (with the
  // optional user_type) and enqueues the verification email; for an existing
  // unverified user it re-issues a code instead (idempotent). New users also
  // get their 1:1 profile row: COMPANY → companies (first + last), USER →
  // users_profile, ADMIN → none.
  async signup(inp: SignupInput): Promise<void> {
    const email = inp.email.toLowerCase().trim();
    if (email === "" || inp.password.length < 8) {
      throw errValidation;
    }
    const userType = normalizeUserType(inp.userType);
    if (userType === null) {
      throw errValidation;
    }
    const now = this.clock.now();
    const existing = await this.store.getUserByEmail(email);
    if (existing !== null && existing.isVerified) {
      throw errEmailTakenActive;
    }
    let userId: string;
    let lang: string;
    let row: UserRow | null = null;
    if (existing !== null) {
      userId = existing.id;
      lang = defaultLang(existing.language);
    } else {
      const hash = await hashPassword(inp.password, this.pepper);
      userId = uuidGen.newId();
      lang = defaultLang(inp.language);
      row = {
        id: userId,
        email,
        firstName: inp.firstName,
        lastName: inp.lastName,
        passwordHash: hash,
        role: "USER",
        userType,
        language: lang,
        isActive: true,
        isVerified: false,
      };
    }
    if (row !== null) {
      await this.store.createUser(row);
      await this.createProfile(userId, userType, inp.firstName, inp.lastName);
    }
    // Re-issue regardless of path: one pending code, always freshly sent.
    const code = newCode(6);
    const v: Verification = {
      id: uuidGen.newId(),
      userId,
      token: code,
      expiresAt: new Date(now.getTime() + this.verifyTtlMs),
      used: false,
      attempts: 0,
    };
    await this.store.createVerification(v);
    await this.enqueueVerification(email, inp.firstName, lang, code);
  }

  // verifyEmail consumes the 6-digit code and flips is_verified. Wrong codes
  // bump the attempt counter; past maxCodeAttempts the code reads as invalid,
  // so 24h of guessing cannot exhaust the 6-digit space.
  async verifyEmail(emailAddr: string, code: string): Promise<void> {
    emailAddr = emailAddr.toLowerCase().trim();
    if (code === "") {
      throw errCodeInvalid;
    }
    const user = await this.store.getUserByEmail(emailAddr);
    if (user === null) {
      throw errCodeInvalid;
    }
    const v = await this.store.latestVerification(user.id);
    if (v === null) {
      throw errCodeInvalid;
    }
    if (v.used) {
      throw errCodeUsed;
    }
    if (this.clock.now().getTime() > v.expiresAt.getTime()) {
      throw errCodeExpired;
    }
    if (v.attempts >= maxCodeAttempts) {
      throw errCodeInvalid;
    }
    if (!tokenEq(v.token, code)) {
      await this.store.bumpVerification(v.id);
      throw errCodeInvalid;
    }
    const consumed = await this.store.useVerification(v.id, v.token);
    if (!consumed) {
      throw errCodeUsed;
    }
    await this.store.setEmailVerified(user.id);
  }

  // resendVerification creates a fresh code for an unverified user; unknown
  // and already-verified emails answer the same way (no existence leak) —
  // always 200, nothing enqueued when unknown/verified.
  async resendVerification(emailAddr: string): Promise<void> {
    emailAddr = emailAddr.toLowerCase().trim();
    const user = await this.store.getUserByEmail(emailAddr);
    if (user === null || user.isVerified) {
      return;
    }
    const code = newCode(6);
    const v: Verification = {
      id: uuidGen.newId(),
      userId: user.id,
      token: code,
      expiresAt: new Date(this.clock.now().getTime() + this.verifyTtlMs),
      used: false,
      attempts: 0,
    };
    await this.store.createVerification(v);
    await this.enqueueVerification(user.email, user.firstName, user.language, code);
  }

  // enqueueVerification posts the localized verify_email message to the
  // outbox; locale defaults to "en" — templates.ts resolves the language.
  private async enqueueVerification(
    emailAddr: string,
    firstName: string,
    lang: string,
    code: string,
  ): Promise<void> {
    const locale = lang === "" ? "en" : lang;
    await this.emails.enqueue({
      to: emailAddr,
      templateKey: "verify_email",
      vars: { first_name: firstName, code, locale },
    });
  }

  // createProfile seeds the 1:1 profile row for the user's type; ADMIN needs
  // none. ponytail: runs after createUser on a plain pool (no tx) — a failed
  // insert leaves an unverified user without its profile and re-signup never
  // re-seeds it; wrap the insert path in a store-level tx only if that window
  // actually bites.
  private async createProfile(
    userId: string,
    userType: string,
    firstName: string,
    lastName: string,
  ): Promise<void> {
    const name = `${firstName} ${lastName}`.trim();
    if (userType === UserTypeUser) {
      await this.store.createUserProfile(userId, name);
      return;
    }
    if (userType === UserTypeCompany) {
      await this.store.createCompanyProfile(userId, name);
    }
  }
}

// defaultLang resolves the user language; empty => "en".
const defaultLang = (lang: string): string => (lang === "" ? "en" : lang);

// tokenEq is the constant-time code comparison (subtle.ConstantTimeCompare
// analog); length differences compare as unequal immediately, like Go's.
const tokenEq = (a: string, b: string): boolean => {
  const ab = Buffer.from(a, "utf8");
  const bb = Buffer.from(b, "utf8");
  return ab.length === bb.length && crypto.timingSafeEqual(ab, bb);
};
