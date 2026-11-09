// Acceptance fakes: one shared in-memory user table behind the six store
// adapters — slices never share models, so each port projects its own row
// shape over this one table (Godog fakes.go parity).

import { hashPassword } from "#platform/authx.js";
import { TestPepper, TestClock, nextID } from "../testutil.js";
import type { UserRow as SignupUser, Verification } from "#features/authsignup/domain.js";
import type { Store as SignupStorePort } from "#features/authsignup/ports.js";
import type {
  UserRow as SessionUserRow,
  SessionRow as SessRow,
  SessionUser as SessUser,
  MultiIPRow,
} from "#features/authsession/domain.js";
import type { Store as SessionStorePort } from "#features/authsession/ports.js";
import type { UpdateMeInput } from "#features/authsession/domain.js";
import type { User as PassUser, Reset as PassReset } from "#features/authpassword/domain.js";
import type { Store as PassStorePort } from "#features/authpassword/ports.js";
import type { User as OauthUser, SessionRow as OauthSessRow } from "#features/authoauth/domain.js";
import type { Store as OauthStorePort } from "#features/authoauth/ports.js";
import type { Row as AdminRow, Patch as AdminPatch } from "#features/adminusers/domain.js";
import type { Store as AdminStorePort } from "#features/adminusers/ports.js";
import type { Widget, WidgetKind } from "#features/widgets/domain.js";
import type { Store as WidgetStorePort } from "#features/widgets/ports.js";
import type { GoogleVerifier, GitHubClient } from "#features/authoauth/ports.js";
import type { GoogleIdentity, GitHubUser } from "#platform/oauthx.js";

// SharedUser is the canonical in-memory user the adapters project.
export interface SharedUser {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  passwordHash: string;
  role: string;
  userType: string;
  language: string;
  googleId: string;
  githubId: string;
  picture: string;
  phone: string;
  address: string;
  postal: string;
  isActive: boolean;
  isBlocked: boolean;
  isVerified: boolean;
  retryCount: number;
  createdAt: Date;
  lastLogin: Date;
}

// UserTable is the single in-memory table behind the six store adapters.
export class UserTable {
  clock: TestClock;
  users = new Map<string, SharedUser>();
  emailId = new Map<string, string>();
  order: string[] = [];
  verifs: Verification[] = [];
  Resets: PassReset[] = [];
  sessions = new Map<string, SessRow>();
  profiles = new Map<string, string>();
  widgets = new Map<string, Widget>();
  order2: string[] = [];
  audit: Array<{
    actor: string;
    action: string;
    entity: string;
    entityId: string;
    payload: string;
    ip: string;
    ua: string;
  }> = [];

  constructor(clock: TestClock) {
    this.clock = clock;
  }

  // seedUser inserts a user row directly, bypassing the HTTP API (feature
  // Backgrounds depend on it).
  async seedUser(email: string, password: string, mod?: (r: SharedUser) => void): Promise<void> {
    const rec: SharedUser = {
      id: nextID(),
      email,
      firstName: "",
      lastName: "",
      passwordHash: await hashPassword(password, TestPepper),
      role: "USER",
      userType: "COMPANY",
      language: "en",
      googleId: "",
      githubId: "",
      picture: "",
      phone: "",
      address: "",
      postal: "",
      isActive: true,
      isBlocked: false,
      isVerified: false,
      retryCount: 0,
      createdAt: this.clock.now(),
      lastLogin: new Date(0),
    };
    if (mod) {
      mod(rec);
    }
    this.users.set(rec.id, rec);
    this.emailId.set(email, rec.id);
    this.order.push(rec.id);
  }

  // findEmail resolves a user by email.
  findEmail(email: string): SharedUser | undefined {
    const id = this.emailId.get(email);
    return id === undefined ? undefined : this.users.get(id);
  }

  // sessionTouch marks one session's active flag in place.
  sessionTouch(id: string, active: boolean): void {
    const s = this.sessions.get(id);
    if (s !== undefined) {
      s.isActive = active;
    }
  }
}

// ---- signup adapter -------------------------------------------------------

export class SignupStore implements SignupStorePort {
  constructor(private readonly t: UserTable) {}

  async getUserByEmail(email: string): Promise<SignupUser | null> {
    const r = this.t.findEmail(email);
    if (r === undefined) {
      return null;
    }
    return this.row(r);
  }

  async createUser(row: SignupUser): Promise<void> {
    const rec: SharedUser = {
      id: row.id,
      email: row.email,
      firstName: row.firstName,
      lastName: row.lastName,
      passwordHash: row.passwordHash,
      role: row.role,
      userType: row.userType,
      language: row.language,
      isActive: row.isActive,
      isVerified: row.isVerified,
      googleId: "",
      githubId: "",
      picture: "",
      phone: "",
      address: "",
      postal: "",
      isBlocked: false,
      retryCount: 0,
      createdAt: this.t.clock.now(),
      lastLogin: new Date(0),
    };
    this.t.users.set(rec.id, rec);
    this.t.emailId.set(row.email, rec.id);
    this.t.order.push(rec.id);
  }

  // row projects the shared record onto the signup view (defaults fill the
  // fields that row never carries).
  private row(r: SharedUser): SignupUser {
    return {
      id: r.id,
      email: r.email,
      firstName: r.firstName,
      lastName: r.lastName,
      passwordHash: r.passwordHash,
      role: r.role,
      userType: r.userType,
      language: r.language,
      isActive: r.isActive,
      isVerified: r.isVerified,
    };
  }

  async createVerification(v: Verification): Promise<void> {
    this.t.verifs.push(v);
  }

  async latestVerification(userId: string): Promise<Verification | null> {
    for (let i = this.t.verifs.length - 1; i >= 0; i--) {
      const v = this.t.verifs[i]!;
      if (v.userId === userId) {
        return v;
      }
    }
    return null;
  }

  // useVerification atomically consumes the matching pending code.
  async useVerification(vId: string, token: string): Promise<boolean> {
    const v = this.t.verifs.find((x) => x.id === vId);
    if (v === undefined || v.used || v.token !== token) {
      return false;
    }
    v.used = true;
    return true;
  }

  async bumpVerification(vId: string): Promise<void> {
    const v = this.t.verifs.find((x) => x.id === vId);
    if (v !== undefined) {
      v.attempts += 1;
    }
  }

  async setEmailVerified(userId: string): Promise<void> {
    this.rowById(userId).isVerified = true;
  }

  async createCompanyProfile(userId: string, companyName: string): Promise<void> {
    this.t.profiles.set(userId, companyName);
  }

  async createUserProfile(userId: string, fullName: string): Promise<void> {
    this.t.profiles.set(userId, fullName);
  }

  // rowById resolves a user by id (fake-internal).
  private rowById(id: string): SharedUser {
    const r = this.t.users.get(id);
    if (r === undefined) {
      throw new Error(`fake: unknown user ${id}`);
    }
    return r;
  }
}

// ---- session adapter ------------------------------------------------------

export class SessionStore implements SessionStorePort {
  constructor(private readonly t: UserTable) {}

  async getUserByEmail(email: string): Promise<SessionUserRow | null> {
    const r = this.t.findEmail(email);
    return r === undefined ? null : this.srow(r);
  }

  async getUserByID(id: string): Promise<SessionUserRow | null> {
    const r = this.t.users.get(id);
    return r === undefined ? null : this.srow(r);
  }

  // srow projects the full profile (fakes.go sessionRow parity: both getters
  // are full views; the zero-value login-view quirks live in the SQL adapter).
  private srow(r: SharedUser): SessionUserRow {
    return {
      id: r.id,
      email: r.email,
      firstName: r.firstName,
      lastName: r.lastName,
      passwordHash: r.passwordHash,
      role: r.role,
      userType: r.userType,
      language: r.language,
      phone: r.phone === "" ? null : r.phone,
      picture: r.picture === "" ? null : r.picture,
      address: r.address === "" ? null : r.address,
      postal: r.postal === "" ? null : r.postal,
      subscription: null,
      isActive: r.isActive,
      isBlocked: r.isBlocked,
      isVerified: r.isVerified,
      retryCount: r.retryCount,
      lastLogin: r.lastLogin.getTime() === 0 ? null : r.lastLogin,
      createdAt: r.createdAt,
      updatedAt: new Date(0),
      googleLinked: r.googleId !== "",
      githubLinked: r.githubId !== "",
    };
  }

  // updateMe mirrors the SQL: a present field (even "") overwrites, then the
  // profile is re-read (update-then-GetMe parity).
  async updateMe(userId: string, inp: UpdateMeInput): Promise<SessionUserRow | null> {
    const r = this.t.users.get(userId);
    if (r === undefined) {
      return null;
    }
    if (inp.firstName !== undefined) {
      r.firstName = inp.firstName;
    }
    if (inp.lastName !== undefined) {
      r.lastName = inp.lastName;
    }
    if (inp.phone !== undefined) {
      r.phone = inp.phone;
    }
    if (inp.picture !== undefined) {
      r.picture = inp.picture;
    }
    if (inp.language !== undefined) {
      r.language = inp.language;
    }
    if (inp.address !== undefined) {
      r.address = inp.address;
    }
    if (inp.postal !== undefined) {
      r.postal = inp.postal;
    }
    return this.srow(r);
  }

  // afterFailedLogin increments SQL-side and reports whether the user just
  // crossed the lock threshold.
  async afterFailedLogin(userId: string, lockAt: number): Promise<boolean> {
    const r = this.t.users.get(userId);
    if (r === undefined) {
      return false;
    }
    r.retryCount += 1;
    const blocked = r.retryCount >= lockAt;
    if (blocked) {
      r.isBlocked = true;
    }
    return blocked;
  }

  async afterSuccessfulLogin(userId: string): Promise<void> {
    const r = this.t.users.get(userId);
    if (r !== undefined) {
      r.retryCount = 0;
      r.lastLogin = this.t.clock.now();
    }
  }

  // createSession stamps the DB defaults (created_at/last_activity = now()).
  async createSession(s: SessRow): Promise<void> {
    const row = s.createdAt.getTime() === 0 ? { ...s, createdAt: this.t.clock.now() } : s;
    const final =
      row.lastActivity.getTime() === 0 ? { ...row, lastActivity: this.t.clock.now() } : row;
    this.t.sessions.set(final.id, final);
  }

  // activeSessionByToken joins the owner's current state: the rotation view
  // only sees active, unexpired rows (SQL parity).
  async activeSessionByToken(token: string): Promise<SessUser | null> {
    const now = this.t.clock.now().getTime();
    for (const row of this.t.sessions.values()) {
      if (row.refreshToken !== token || !row.isActive || row.expiresAt.getTime() <= now) {
        continue;
      }
      const r = this.t.users.get(row.userId);
      if (r === undefined) {
        continue;
      }
      return {
        ...row,
        email: r.email,
        role: r.role,
        userActive: r.isActive,
        isBlocked: r.isBlocked,
        isVerified: r.isVerified,
      };
    }
    return null;
  }

  // deactivateSession claims the refresh row: true only when still active.
  async deactivateSession(sessionId: string): Promise<boolean> {
    const row = this.t.sessions.get(sessionId);
    if (row === undefined || !row.isActive) {
      return false;
    }
    this.t.sessionTouch(sessionId, false);
    return true;
  }

  // rotateSession mirrors the SQL: id + old token + active + unexpired
  // rotates in place, keeping id/created_at/expires_at.
  async rotateSession(sessionId: string, oldToken: string, s: SessRow): Promise<boolean> {
    const now = this.t.clock.now().getTime();
    const cur = this.t.sessions.get(sessionId);
    if (
      cur === undefined ||
      cur.refreshToken !== oldToken ||
      !cur.isActive ||
      cur.expiresAt.getTime() <= now
    ) {
      return false;
    }
    this.t.sessions.set(sessionId, {
      ...s,
      id: cur.id,
      userId: cur.userId,
      createdAt: cur.createdAt,
      expiresAt: cur.expiresAt,
      isActive: true,
      lastActivity: this.t.clock.now(),
    });
    return true;
  }

  async deactivateByTokenForUser(userId: string, token: string): Promise<void> {
    for (const row of this.t.sessions.values()) {
      if (row.userId === userId && row.refreshToken === token && row.isActive) {
        this.t.sessionTouch(row.id, false);
      }
    }
  }

  async listSessionsByUser(userId: string, activeOnly: boolean): Promise<SessRow[]> {
    const out: SessRow[] = [];
    for (const row of this.t.sessions.values()) {
      if (row.userId === userId && (!activeOnly || row.isActive)) {
        out.push(row);
      }
    }
    return out.sort((a, b) => b.lastActivity.getTime() - a.lastActivity.getTime());
  }

  async currentSession(userId: string): Promise<SessRow | null> {
    let best: SessRow | null = null;
    for (const row of this.t.sessions.values()) {
      if (row.userId !== userId || !row.isActive) {
        continue;
      }
      if (best === null || row.lastActivity.getTime() > best.lastActivity.getTime()) {
        best = row;
      }
    }
    return best;
  }

  // revokeSessionForUser claims one active session of the user.
  async revokeSessionForUser(userId: string, sessionId: string): Promise<boolean> {
    const row = this.t.sessions.get(sessionId);
    if (row === undefined || row.userId !== userId || !row.isActive) {
      return false;
    }
    this.t.sessionTouch(sessionId, false);
    return true;
  }

  async revokeAllForUser(userId: string): Promise<number> {
    let n = 0;
    for (const row of this.t.sessions.values()) {
      if (row.userId === userId && row.isActive) {
        this.t.sessionTouch(row.id, false);
        n += 1;
      }
    }
    return n;
  }

  async countExpiredActive(): Promise<number> {
    const now = this.t.clock.now().getTime();
    let n = 0;
    for (const row of this.t.sessions.values()) {
      if (row.isActive && row.expiresAt.getTime() <= now) {
        n += 1;
      }
    }
    return n;
  }

  // multiIPSessions reports users with >3 distinct IPs among the active
  // sessions since the window start (reference _MULTI_IP_THRESHOLD).
  async multiIPSessions(since: Date): Promise<MultiIPRow[]> {
    const saw = new Map<string, Set<string>>();
    for (const row of this.t.sessions.values()) {
      if (!row.isActive || row.lastActivity.getTime() < since.getTime() || row.ipAddress === "") {
        continue;
      }
      const ips = saw.get(row.userId) ?? new Set<string>();
      ips.add(row.ipAddress);
      saw.set(row.userId, ips);
    }
    const out: MultiIPRow[] = [];
    for (const [userId, ips] of saw) {
      if (ips.size > 3) {
        out.push({ userId, ipCount: ips.size });
      }
    }
    return out.sort((a, b) => (a.userId < b.userId ? -1 : 1));
  }
}

// ---- password adapter -----------------------------------------------------

export class PasswordStore implements PassStorePort {
  constructor(private readonly t: UserTable) {}

  async getUserByEmail(email: string): Promise<PassUser | null> {
    const r = this.t.findEmail(email);
    return r === undefined
      ? null
      : { id: r.id, email: r.email, firstName: r.firstName, password: "", language: r.language };
  }

  async getUserByID(id: string): Promise<PassUser | null> {
    const r = this.t.users.get(id);
    return r === undefined
      ? null
      : { id: r.id, email: r.email, firstName: "", password: r.passwordHash, language: r.language };
  }

  async countResetsSince(email: string, since: Date): Promise<number> {
    let n = 0;
    for (const r of this.t.Resets) {
      if (r.email === email && r.createdAt.getTime() >= since.getTime()) {
        n += 1;
      }
    }
    return n;
  }

  async createReset(r: PassReset): Promise<void> {
    this.t.Resets.push({ ...r, createdAt: this.t.clock.now() });
  }

  async latestReset(email: string): Promise<PassReset | null> {
    for (let i = this.t.Resets.length - 1; i >= 0; i--) {
      const r = this.t.Resets[i]!;
      if (r.email === email) {
        return r;
      }
    }
    return null;
  }

  // markResetUsed consumes the code atomically: token match + unused only.
  async markResetUsed(id: string, token: string): Promise<boolean> {
    const r = this.t.Resets.find((x) => x.id === id);
    if (r === undefined || r.token !== token || r.used) {
      return false;
    }
    r.used = true;
    return true;
  }

  async bumpReset(id: string): Promise<void> {
    const r = this.t.Resets.find((x) => x.id === id);
    if (r !== undefined) {
      r.attempts += 1;
    }
  }

  async setPasswordAndVerify(id: string, hash: string): Promise<void> {
    const r = this.t.users.get(id);
    if (r !== undefined) {
      r.passwordHash = hash;
      r.isVerified = true;
    }
  }

  async setPassword(id: string, hash: string): Promise<void> {
    const r = this.t.users.get(id);
    if (r !== undefined) {
      r.passwordHash = hash;
    }
  }

  async revokeAllSessions(userId: string): Promise<void> {
    for (const row of this.t.sessions.values()) {
      if (row.userId === userId && row.isActive) {
        this.t.sessionTouch(row.id, false);
      }
    }
  }

  // revokeOtherSessions keeps the caller's session (jti == sessions.id).
  async revokeOtherSessions(userId: string, sessionId: string): Promise<void> {
    for (const row of this.t.sessions.values()) {
      if (row.userId === userId && row.id !== sessionId && row.isActive) {
        this.t.sessionTouch(row.id, false);
      }
    }
  }
}

// ---- admin adapter --------------------------------------------------------

export class AdminStore implements AdminStorePort {
  constructor(private readonly t: UserTable) {}

  private arow(r: SharedUser): AdminRow {
    return {
      id: r.id,
      email: r.email,
      firstName: r.firstName,
      lastName: r.lastName,
      role: r.role,
      userType: r.userType,
      isActive: r.isActive,
      isBlocked: r.isBlocked,
      isVerified: r.isVerified,
      language: r.language,
      createdAt: r.createdAt,
      lastLogin: r.lastLogin.getTime() === 0 ? new Date(0) : r.lastLogin,
    };
  }

  async list(limit: number, offset: number): Promise<AdminRow[]> {
    // ponytail: offset window over the insertion order; <100 users per scenario.
    const out: AdminRow[] = [];
    for (let i = offset; i < this.t.order.length && out.length < limit; i++) {
      const r = this.t.users.get(this.t.order[i]!);
      if (r !== undefined) {
        out.push(this.arow(r));
      }
    }
    return out;
  }

  async count(): Promise<number> {
    return this.t.order.length;
  }

  async get(id: string): Promise<AdminRow | null> {
    const r = this.t.users.get(id);
    return r === undefined ? null : this.arow(r);
  }

  async getByEmail(email: string): Promise<AdminRow | null> {
    const e = email.toLowerCase();
    for (const r of this.t.users.values()) {
      if (r.email.toLowerCase() === e) {
        return this.arow(r);
      }
    }
    return null;
  }

  async update(id: string, patch: AdminPatch): Promise<void> {
    const r = this.t.users.get(id);
    if (r === undefined) {
      return;
    }
    if (patch.isActive !== null) {
      r.isActive = patch.isActive;
    }
    if (patch.isBlocked !== null) {
      r.isBlocked = patch.isBlocked;
    }
    if (patch.role !== "") {
      r.role = patch.role;
    }
  }

  async upgradeToAdmin(id: string): Promise<void> {
    const r = this.t.users.get(id);
    if (r !== undefined) {
      r.role = "ADMIN";
      r.userType = "ADMIN";
    }
  }

  async writeAudit(
    actor: string,
    action: string,
    entity: string,
    entityId: string,
    payload: string,
    ip: string,
    ua: string,
  ): Promise<void> {
    this.t.audit.push({ actor, action, entity, entityId, payload, ip, ua });
  }
}

// ---- oauth adapter --------------------------------------------------------

export class OauthStore implements OauthStorePort {
  constructor(private readonly t: UserTable) {}

  private orow(r: SharedUser): OauthUser {
    return {
      id: r.id,
      email: r.email,
      firstName: r.firstName,
      lastName: r.lastName,
      password: "",
      picture: r.picture,
      role: r.role,
      userType: r.userType,
      language: r.language,
      googleId: r.googleId,
      githubId: r.githubId,
      isActive: r.isActive,
      isBlocked: r.isBlocked,
      isVerified: r.isVerified,
    };
  }

  async getUserByGoogleID(googleID: string): Promise<OauthUser | null> {
    for (const id of this.t.order) {
      const r = this.t.users.get(id!);
      if (r !== undefined && r.googleId !== "" && r.googleId === googleID) {
        return this.orow(r);
      }
    }
    return null;
  }

  async getUserByGithubID(githubID: string): Promise<OauthUser | null> {
    for (const id of this.t.order) {
      const r = this.t.users.get(id!);
      if (r !== undefined && r.githubId !== "" && r.githubId === githubID) {
        return this.orow(r);
      }
    }
    return null;
  }

  async getUserByEmail(email: string): Promise<OauthUser | null> {
    const r = this.t.findEmail(email);
    return r === undefined ? null : this.orow(r);
  }

  async getUserByID(id: string): Promise<OauthUser | null> {
    const r = this.t.users.get(id);
    return r === undefined ? null : this.orow(r);
  }

  async createUser(u: OauthUser): Promise<void> {
    const rec: SharedUser = {
      id: u.id,
      email: u.email,
      firstName: u.firstName,
      lastName: u.lastName,
      passwordHash: u.password,
      role: u.role,
      userType: u.userType,
      language: u.language,
      googleId: u.googleId,
      githubId: u.githubId,
      picture: u.picture ?? "",
      phone: "",
      address: "",
      postal: "",
      isActive: u.isActive,
      isBlocked: u.isBlocked,
      isVerified: u.isVerified,
      retryCount: 0,
      createdAt: this.t.clock.now(),
      lastLogin: new Date(0),
    };
    this.t.users.set(rec.id, rec);
    this.t.emailId.set(rec.email, rec.id);
    this.t.order.push(rec.id);
  }

  // setProvider applies one provider link (verify + picture-if-empty parity
  // with the SQL adapters).
  private setProvider(
    userID: string,
    field: "google" | "github",
    providerID: string,
    picture: string,
  ): void {
    const r = this.rowById(userID);
    if (field === "google") {
      r.googleId = providerID;
    } else {
      r.githubId = providerID;
    }
    r.isVerified = true;
    if (r.picture === "") {
      r.picture = picture;
    }
  }

  private rowById(id: string): SharedUser {
    const r = this.t.users.get(id);
    if (r === undefined) {
      throw new Error(`fake: unknown user ${id}`);
    }
    return r;
  }

  async linkGoogle(userID: string, providerID: string, picture: string): Promise<void> {
    this.setProvider(userID, "google", providerID, picture);
  }

  async linkGithub(userID: string, providerID: string, picture: string): Promise<void> {
    this.setProvider(userID, "github", providerID, picture);
  }

  // githubOnly attaches the github id with no side effects (link flow).
  async githubOnly(userID: string, githubID: string): Promise<void> {
    this.rowById(userID).githubId = githubID;
  }

  // saveGithubLogin refreshes the login-flow identity (email upgrade decided
  // in the use case; verified mark + picture-if-empty here).
  async saveGithubLogin(userID: string, email: string, picture: string): Promise<void> {
    const r = this.rowById(userID);
    if (email !== "") {
      r.email = email;
      this.t.emailId.set(email, userID);
    }
    r.isVerified = true;
    if (r.picture === "") {
      r.picture = picture;
    }
  }

  // createSession rides the shared sessions table, DB-default stamps applied.
  async createSession(s: OauthSessRow): Promise<void> {
    const now = this.t.clock.now();
    this.t.sessions.set(s.id, {
      id: s.id,
      userId: s.userId,
      refreshToken: s.refreshToken,
      ipAddress: s.ipAddress,
      userAgent: s.userAgent,
      expiresAt: s.expiresAt,
      isActive: s.isActive,
      createdAt: now,
      lastActivity: now,
    });
  }
}

// ---- widgets adapter ------------------------------------------------------

export class WidgetStore implements WidgetStorePort {
  constructor(private readonly t: UserTable) {}

  async insert(w: Widget): Promise<void> {
    this.t.widgets.set(w.id, w);
    this.t.order2.push(w.id);
  }

  async list(ownerId: string, limit: number, offset: number): Promise<Widget[]> {
    const out: Widget[] = [];
    for (let i = offset; i < this.t.order2.length && out.length < limit; i++) {
      const w = this.t.widgets.get(this.t.order2[i]!);
      if (w !== undefined && w.ownerUserId === ownerId) {
        out.push(w);
      }
    }
    return out;
  }

  async get(id: string, ownerId: string): Promise<Widget | null> {
    const w = this.t.widgets.get(id);
    return w === undefined || w.ownerUserId !== ownerId ? null : w;
  }

  async update(id: string, ownerId: string, name: string, kind: WidgetKind): Promise<void> {
    const w = this.t.widgets.get(id);
    if (w !== undefined && w.ownerUserId === ownerId) {
      w.name = name;
      w.kind = kind;
    }
  }

  async delete(id: string, _ownerId: string): Promise<void> {
    this.t.widgets.delete(id);
  }
}

// ---- fake providers -------------------------------------------------------

// FakeGoogle answers a verified identity for any @ex.com credential — the
// address itself is the token (the real verifier checks RS256/JWKS).
export class FakeGoogle implements GoogleVerifier {
  async verify(credential: string): Promise<GoogleIdentity> {
    const at = credential.indexOf("@");
    const local = at === -1 ? "" : credential.slice(0, at);
    if (at === -1 || credential.slice(at + 1) !== "ex.com" || local === "") {
      throw new Error("invalid credential");
    }
    return {
      sub: `google-${local}`,
      email: credential,
      givenName: local,
      familyName: "Google",
      picture: `https://gravatar.test/${local}`,
    };
  }
}

// FakeGitHub maps fixed codes to fixed identities: good-code provisions
// fresh@ex.com, ada-code links ada@ex.com.
export class FakeGitHub implements GitHubClient {
  readonly codes = new Map<string, GitHubUser>([
    [
      "good-code",
      { id: "gh-fresh", login: "freshgh", name: "", email: "fresh@ex.com", avatarURL: "" },
    ],
    ["ada-code", { id: "gh-ada", login: "ada", name: "Ada L", email: "ada@ex.com", avatarURL: "" }],
  ]);

  authorize(clientID: string, redirectURI: string, state: string): string {
    const q = new URLSearchParams({
      client_id: clientID,
      redirect_uri: redirectURI,
      scope: "user:email",
    });
    if (state !== "") {
      q.set("state", state);
    }
    return `https://github.com/login/oauth/authorize?${q.toString()}`;
  }

  async exchange(code: string): Promise<GitHubUser> {
    const u = this.codes.get(code);
    if (u === undefined) {
      throw new Error("bad_verification_code");
    }
    return u;
  }
}
