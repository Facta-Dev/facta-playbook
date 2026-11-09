// Features authoauth postgres adapts the Kysely database to this slice's
// Store port (the sqlc adapter's analog). NULL provider ids read as "".

import { Kysely, PostgresDialect } from "kysely";
import type pg from "pg";
import type { User, SessionRow } from "./domain.js";
import type { Store } from "./ports.js";
import {
  oauthGetUserByGoogleID,
  oauthGetUserByGithubID,
  oauthGetUserByEmail,
  oauthGetUserByID,
  oauthCreateUser,
  oauthLinkGoogle,
  oauthLinkGithub,
  oauthGithubOnly,
  oauthSaveGithubLogin,
  oauthCreateSession,
  type Database,
  type OauthUserSel,
} from "./queries.js";

// Postgres adapts the typed queries to this slice's Store port.
export class Postgres implements Store {
  readonly #db: Kysely<Database>;

  constructor(pool: pg.Pool) {
    this.#db = new Kysely<Database>({ dialect: new PostgresDialect({ pool }) });
  }

  async getUserByGoogleID(googleID: string): Promise<User | null> {
    const r = await oauthGetUserByGoogleID(this.#db, googleID);
    return r === null ? null : rowToUser(r);
  }

  async getUserByGithubID(githubID: string): Promise<User | null> {
    const r = await oauthGetUserByGithubID(this.#db, githubID);
    return r === null ? null : rowToUser(r);
  }

  async getUserByEmail(id: string): Promise<User | null> {
    const r = await oauthGetUserByEmail(this.#db, id);
    return r === null ? null : rowToUser(r);
  }

  async getUserByID(id: string): Promise<User | null> {
    const r = await oauthGetUserByID(this.#db, id);
    return r === null ? null : rowToUser(r);
  }

  async createUser(u: User): Promise<void> {
    await oauthCreateUser(this.#db, u);
  }

  async linkGoogle(userID: string, providerID: string, picture: string): Promise<void> {
    await oauthLinkGoogle(this.#db, { userID, providerID, picture });
  }

  async linkGithub(userID: string, providerID: string, picture: string): Promise<void> {
    await oauthLinkGithub(this.#db, { userID, providerID, picture });
  }

  async githubOnly(userID: string, githubID: string): Promise<void> {
    await oauthGithubOnly(this.#db, { userID, githubID });
  }

  async saveGithubLogin(userID: string, email: string, picture: string): Promise<void> {
    await oauthSaveGithubLogin(this.#db, { userID, email, picture });
  }

  async createSession(s: SessionRow): Promise<void> {
    await oauthCreateSession(this.#db, s);
  }
}

// rowToUser maps the oauth users view: the random provisioning password is
// never selected ("" reads as unusable in the use case), provider ids NULL
// reads as "" (= not linked).
const rowToUser = (r: OauthUserSel): User => ({
  id: r.id,
  email: r.email,
  firstName: r.first_name,
  lastName: r.last_name,
  password: "",
  picture: r.profile_picture,
  role: r.role,
  userType: r.user_type,
  language: r.language,
  googleId: r.google_id ?? "",
  githubId: r.github_id ?? "",
  isActive: r.is_active,
  isBlocked: r.is_blocked,
  isVerified: r.is_verified,
});
