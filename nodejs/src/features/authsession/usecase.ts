// Features authsession use case: login / refresh / logout / me / update-me /
// sessions / anomaly report. Port of the Go template's authsession/usecase.go.

import { checkPassword } from "#platform/authx.js";
import { type Clock } from "#shared/clockx.js";
import { newToken, uuidGen } from "#shared/idgen.js";
import type { Role } from "#shared/rolex.js";
import {
  type UserRow,
  type SessionRow,
  type Anomaly,
  type MultiIPRow,
  type UpdateMeInput,
  errInvalidCredentials,
  errInvalidRefresh,
  errAccountInactive,
  errAccountBlocked,
  errEmailNotVerified,
  errUserNotFound,
  errSignedUpWithSocial,
} from "./domain.js";
import { withDetail, E, CodeUnprocessable, Validation } from "#shared/apperr.js";
import type { Store, TokenMinter } from "./ports.js";

export { type UpdateMeInput } from "./domain.js";

// LoginInput carries request + request metadata for the session row.
export interface LoginInput {
  email: string;
  password: string;
  ipAddress: string;
  userAgent: string;
}

// Result is what the HTTP adapter turns into the AuthTokens wire shape.
export interface Result {
  accessToken: string;
  refreshToken: string;
  expiresAt: Date;
}

// UseCase implements login / refresh / logout / me / update-me / sessions.
export class UseCase {
  constructor(
    private readonly store: Store,
    private readonly minter: TokenMinter,
    private readonly clock: Clock,
    private readonly pepper: string,
    private readonly accessTtlMs: number,
    private readonly refreshTtlMs: number,
    private readonly maxLoginRetry: number,
  ) {}

  // login counts failures with the retry lock, and — only after the password
  // verifies (parity: the state gates are unreachable with a wrong password,
  // so a probe cannot reveal an account's existence or state) — the account
  // state gates, then issues the token pair. A wrong password on a
  // social-provisioned account guides toward that provider and still counts
  // against the lock.
  async login(inp: LoginInput): Promise<Result> {
    const user = await this.store.getUserByEmail(inp.email);
    if (user === null) {
      throw errInvalidCredentials;
    }
    try {
      await checkPassword(user.passwordHash, inp.password, this.pepper);
    } catch {
      const blocked = await this.store.afterFailedLogin(user.id, this.maxLoginRetry);
      if (blocked) {
        throw errAccountBlocked;
      }
      if (user.googleLinked || user.githubLinked) {
        throw withDetail(
          errSignedUpWithSocial,
          socialGuidance(user.googleLinked, user.githubLinked),
        );
      }
      throw errInvalidCredentials;
    }
    if (user.isBlocked) {
      throw errAccountBlocked;
    }
    if (!user.isActive) {
      throw errAccountInactive;
    }
    if (!user.isVerified) {
      throw errEmailNotVerified;
    }
    await this.store.afterSuccessfulLogin(user.id);
    const session = newSession(
      inp.ipAddress,
      inp.userAgent,
      user.id,
      this.clock.now(),
      this.refreshTtlMs,
    );
    await this.store.createSession(session);
    return mintFor(user, session.id, session.refreshToken, this.minter, this.accessTtlMs);
  }

  // refresh re-checks the account state, then rotates the session row in
  // place: the row keeps its id and expires_at (fixed absolute lifetime —
  // a session never lives past the refresh TTL set at login), and the CAS
  // on the old token rejects a replayed token.
  async refresh(token: string, ip: string, ua: string): Promise<Result> {
    const su = await this.store.activeSessionByToken(token);
    if (su === null) {
      throw errInvalidRefresh;
    }
    if (su.isBlocked) {
      await this.store.deactivateSession(su.id);
      throw errAccountBlocked;
    }
    if (!su.userActive) {
      await this.store.deactivateSession(su.id);
      throw errAccountInactive;
    }
    if (!su.isVerified) {
      await this.store.deactivateSession(su.id);
      throw errEmailNotVerified;
    }
    const newTok = newToken();
    const rotated: SessionRow = {
      id: su.id,
      userId: su.userId,
      refreshToken: newTok,
      expiresAt: su.expiresAt,
      ipAddress: ip,
      userAgent: ua,
      isActive: true,
      createdAt: su.createdAt,
      lastActivity: su.lastActivity,
    };
    const claimed = await this.store.rotateSession(su.id, su.refreshToken, rotated);
    if (!claimed) {
      throw errInvalidRefresh;
    }
    return mintFor(
      {
        id: su.userId,
        email: su.email,
        role: su.role,
      } as UserRow,
      su.id,
      newTok,
      this.minter,
      this.accessTtlMs,
    );
  }

  // logout revokes the given refresh token for this user (idempotent).
  async logout(userId: string, refreshToken: string): Promise<void> {
    await this.store.deactivateByTokenForUser(userId, refreshToken);
  }

  // me returns the profile of the authenticated user.
  async me(userId: string): Promise<UserRow> {
    const user = await this.store.getUserByID(userId);
    if (user === null) {
      throw errUserNotFound;
    }
    return user;
  }

  // updateMe applies the non-null fields and re-reads the profile
  // (update-then-GetMe parity). Unknown id → errUserNotFound.
  async updateMe(userId: string, input: UpdateMeInput): Promise<UserRow> {
    const user = await this.store.updateMe(userId, input);
    if (user === null) {
      throw errUserNotFound;
    }
    return user;
  }

  // sessions lists the user's sessions, newest activity first (active-only
  // by default).
  sessions(userId: string, activeOnly: boolean): Promise<SessionRow[]> {
    return this.store.listSessionsByUser(userId, activeOnly);
  }

  // currentSession returns the user's most recent active session.
  currentSession(userId: string): Promise<SessionRow | null> {
    return this.store.currentSession(userId);
  }

  // revokeCurrent claims one active session of the user (CAS on is_active):
  // false when sessionId is not among the user's active sessions — the
  // access token's jti IS the session id, so no body is needed.
  async revokeCurrent(userId: string, sessionId: string): Promise<boolean> {
    if (sessionId === "") {
      return false;
    }
    return this.store.revokeSessionForUser(userId, sessionId);
  }

  // revokeAll deactivates every active session of the user and returns how
  // many were claimed.
  revokeAll(userId: string): Promise<number> {
    return this.store.revokeAllForUser(userId);
  }

  // maxAnomalyWindow caps the anomaly report window: 30 days.
  static readonly maxAnomalyWindowMs = 30 * 24 * 60 * 60 * 1000;

  // anomalies is the admin session report: expired-but-active rows first,
  // then users whose active sessions within the window span more distinct
  // IPs than the SQL threshold (reader aggregate parity).
  async anomalies(windowMs: number): Promise<Anomaly[]> {
    if (windowMs <= 0 || windowMs > UseCase.maxAnomalyWindowMs) {
      throw E(CodeUnprocessable, Validation, "window out of range");
    }
    const expired = await this.store.countExpiredActive();
    const out: Anomaly[] = [];
    if (expired > 0) {
      out.push({ type: "expired_but_active", count: expired, details: null });
    }
    const rows = await this.store.multiIPSessions(new Date(this.clock.now().getTime() - windowMs));
    if (rows.length > 0) {
      out.push({
        type: "multi_ip_sessions",
        count: rows.length,
        details: { user_ids: rows.map((r: MultiIPRow) => r.userId) },
      });
    }
    return out;
  }
}

// newSession builds one durable refresh row.
const newSession = (
  ip: string,
  ua: string,
  userId: string,
  now: Date,
  refreshTtlMs: number,
): SessionRow => ({
  id: uuidGen.newId(),
  userId,
  refreshToken: newToken(),
  expiresAt: new Date(now.getTime() + refreshTtlMs),
  ipAddress: ip,
  userAgent: ua,
  isActive: true,
  // rotation lookup does not select these; coalesced with now() at the wire.
  createdAt: now,
  lastActivity: now,
});

// mintFor issues the access JWT with jti == session ID; change-password's
// "keep the current session" matches on exactly this.
const mintFor = async (
  user: { id: string; email: string; role: string },
  sessionId: string,
  refreshToken: string,
  minter: TokenMinter,
  accessTtlMs: number,
): Promise<Result> => {
  const { token, exp } = await minter.mint(
    { userId: user.id, email: user.email, role: user.role as Role, jti: sessionId },
    accessTtlMs,
  );
  return { accessToken: token, refreshToken, expiresAt: exp };
};

// socialGuidance renders the wrong-password guidance for an account whose
// identity came from Google/GitHub (message parity).
export const socialGuidance = (google: boolean, github: boolean): string => {
  if (google && github) {
    return "This account was created with Google and GitHub. Use one of those to sign in, or reset your password.";
  }
  if (google) {
    return "This account was created with Google. Use Google to sign in, or reset your password to set a password.";
  }
  return "This account was created with GitHub. Use GitHub to sign in, or reset your password to set a password.";
};
