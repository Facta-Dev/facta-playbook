// Features authoauth use case: the google one-tap and github oauth flows on
// top of the session plumbing (same Result/session shape as the email
// login).

import { type GoogleIdentity, type GitHubUser } from "#platform/oauthx.js";
import { type Clock } from "#shared/clockx.js";
import { newToken, uuidGen } from "#shared/idgen.js";
import { type Role } from "#shared/rolex.js";
import {
  type User,
  type SessionRow,
  type Result,
  LinkStatePrefix,
  linkSuccessRedirect,
  errInvalidGoogleToken,
  errGoogleEmailLinked,
  errGithubAlreadyLinked,
  errGoogleNotConfigured,
  errGithubNotConfigured,
  errInvalidLinkState,
  errAccountInactive,
  errAccountBlocked,
  errUserNotFound,
} from "./domain.js";
import type { GoogleVerifier, GitHubClient, LinkState, TokenMinter, Store } from "./ports.js";

// Config carries the slice knobs (module Deps mirrors it verbatim).
export interface Config {
  googleClientID: string;
  googleEnabled: boolean;
  githubClientID: string;
  githubClientSecret: string;
  githubCallbackURL: string;
  githubEnabled: boolean;
  frontendSuccess: string;
  offline: boolean;
  accessTtlMs: number;
  refreshTtlMs: number;
}

// UseCase implements the google one-tap and github oauth flows on top of
// the session plumbing (same Result/session shape as the email login).
export class UseCase {
  private readonly store: Store;
  private readonly googleV: GoogleVerifier;
  private readonly github: GitHubClient;
  private readonly states: LinkState;
  private readonly minter: TokenMinter;
  private readonly clock: Clock;
  private readonly accessTtlMs: number;
  private readonly refreshTtlMs: number;
  private readonly googleID: string;
  private readonly githubID: string;
  private readonly githubSecret: string;
  private readonly githubRedir: string;
  private readonly googleOn: boolean;
  private readonly githubOn: boolean;
  private readonly offline: boolean;

  constructor(
    store: Store,
    google: GoogleVerifier,
    github: GitHubClient,
    states: LinkState,
    minter: TokenMinter,
    clock: Clock,
    c: Config,
  ) {
    this.store = store;
    this.googleV = google;
    this.github = github;
    this.states = states;
    this.minter = minter;
    this.clock = clock;
    this.accessTtlMs = c.accessTtlMs;
    this.refreshTtlMs = c.refreshTtlMs;
    this.googleID = c.googleClientID;
    this.githubID = c.githubClientID;
    this.githubSecret = c.githubClientSecret;
    this.githubRedir = c.githubCallbackURL;
    this.googleOn = c.googleEnabled;
    this.githubOn = c.githubEnabled;
    this.offline = c.offline;
  }

  // Google verifies the One-Tap credential and issues the session: google-id
  // match → email match (verify + link) → provision.
  async google(credential: string, ip: string, ua: string): Promise<Result> {
    if (this.offline || !this.googleOn || this.googleID === "") {
      throw errGoogleNotConfigured;
    }
    let id: GoogleIdentity;
    try {
      id = await this.googleV.verify(credential, this.googleID);
    } catch {
      throw errInvalidGoogleToken;
    }
    const user = await this.findOrLinkGoogle(id);
    return this.startSession(user, ip, ua);
  }

  // findOrLinkGoogle resolves provider identity → account: google-id match,
  // else email match (verify + link), else provision.
  private async findOrLinkGoogle(id: GoogleIdentity): Promise<User> {
    // Provider emails resolve against the stored lowercase convention (the
    // password flows normalize before every lookup/store).
    const email = id.email.toLowerCase().trim();
    let user = await this.store.getUserByGoogleID(id.sub);
    if (user !== null) {
      return user;
    }
    user = await this.store.getUserByEmail(email);
    if (user !== null) {
      if (user.googleId !== "") {
        // a different Google account owns this email
        throw errGoogleEmailLinked;
      }
      await this.store.linkGoogle(user.id, id.sub, id.picture);
      user.googleId = id.sub;
      user.isVerified = true;
      if (user.picture === null || user.picture === "") {
        user.picture = id.picture;
      }
      return user;
    }
    const fresh = newOAuthUser(
      email,
      googleNames(id)[0],
      googleNames(id)[1],
      id.picture,
      id.sub,
      "",
    );
    await this.store.createUser(fresh);
    return fresh;
  }

  // GithubLoginURL returns the authorize redirect for the plain login flow
  // (state passthrough); any refusal is a config problem.
  async githubLoginURL(state: string): Promise<string> {
    await this.githubGate();
    return this.github.authorize(this.githubID, this.githubRedir, state);
  }

  // GithubConnectURL returns the authorize URL carrying state=link:<jwt> so
  // the callback links instead of logging in (authenticated callers only).
  async githubConnectURL(userID: string): Promise<string> {
    await this.githubGate();
    const token = await this.states.mintLink(userID);
    return this.github.authorize(this.githubID, this.githubRedir, LinkStatePrefix + token);
  }

  // GithubLogin processes the login callback: exchange code, resolve the
  // real email, then github-id match (refresh identity) → email match
  // (link) → provision.
  async githubLogin(code: string, ip: string, ua: string): Promise<Result> {
    await this.githubGate();
    const gh = await this.github.exchange(code, this.githubID, this.githubSecret, this.githubRedir);
    const user = await this.findOrLinkGithub(gh);
    return this.startSession(user, ip, ua);
  }

  // GithubLink attaches the freshly exchanged github id to the user named by
  // the link-state JWT; the link flow changes nothing else.
  async githubLink(stateJWT: string, code: string): Promise<void> {
    await this.githubGate();
    let userID: string;
    try {
      userID = await this.states.parseLink(stateJWT);
    } catch {
      throw errInvalidLinkState;
    }
    const gh = await this.github.exchange(code, this.githubID, this.githubSecret, this.githubRedir);
    const other = await this.store.getUserByGithubID(gh.id);
    if (other !== null && other.id !== userID) {
      throw errGithubAlreadyLinked;
    }
    const target = await this.store.getUserByID(userID);
    if (target === null) {
      throw errUserNotFound;
    }
    await this.store.githubOnly(target.id, gh.id);
  }

  // findOrLinkGithub mirrors reference: existing-by-github-id users get
  // their identity refreshed (noreply email upgrade, avatar when empty,
  // verified mark); email collisions are linked; strangers are provisioned.
  private async findOrLinkGithub(gh: GitHubUser): Promise<User> {
    const ghEmail = gh.email.toLowerCase().trim();
    const user = await this.store.getUserByGithubID(gh.id);
    if (user !== null) {
      let email = user.email;
      if (isNoreply(user.email) && ghEmail !== "" && !isNoreply(ghEmail)) {
        email = ghEmail;
      }
      await this.store.saveGithubLogin(user.id, email, gh.avatarURL);
      user.email = email;
      user.isVerified = true;
      if (user.picture === null || user.picture === "") {
        user.picture = gh.avatarURL;
      }
      return user;
    }
    const byEmail = await this.store.getUserByEmail(ghEmail);
    if (byEmail !== null) {
      await this.store.linkGithub(byEmail.id, gh.id, gh.avatarURL);
      byEmail.githubId = gh.id;
      byEmail.isVerified = true;
      if (byEmail.picture === null || byEmail.picture === "") {
        byEmail.picture = gh.avatarURL;
      }
      return byEmail;
    }
    const fresh = newOAuthUser(
      ghEmail,
      githubNames(gh)[0],
      githubNames(gh)[1],
      gh.avatarURL,
      "",
      gh.id,
    );
    await this.store.createUser(fresh);
    return fresh;
  }

  // githubGate is the one refusal rule of every github route.
  // githubGate is the one refusal rule of every github route; the http
  // callback checks it up front so a disabled provider answers the envelope.
  async githubGate(): Promise<void> {
    if (this.offline || !this.githubOn || this.githubID === "" || this.githubSecret === "") {
      throw errGithubNotConfigured;
    }
  }

  // startSession gates account state, stores the refresh session and mints
  // the access token (jti == session id, exactly like the email login).
  private async startSession(user: User, ip: string, ua: string): Promise<Result> {
    if (user.isBlocked) {
      throw errAccountBlocked;
    }
    if (!user.isActive) {
      throw errAccountInactive;
    }
    const refresh = newToken();
    const session: SessionRow = {
      id: uuidGen.newId(),
      userId: user.id,
      refreshToken: refresh,
      ipAddress: ip,
      userAgent: ua,
      expiresAt: new Date(this.clock.now().getTime() + this.refreshTtlMs),
      isActive: true,
    };
    await this.store.createSession(session);
    const { token, exp } = await this.minter.mint(
      { userId: user.id, email: user.email, role: user.role as Role, jti: session.id },
      this.accessTtlMs,
    );
    return { accessToken: token, refreshToken: refresh, expiresAt: exp };
  }
}

// newOAuthUser builds the provisioned social user: auto-verified, COMPANY
// user type (design default), and an unusable random password (the column
// is NOT NULL; the value is never handed to anyone).
const newOAuthUser = (
  email: string,
  first: string,
  last: string,
  picture: string,
  googleID: string,
  githubID: string,
): User => ({
  id: uuidGen.newId(),
  email,
  firstName: first,
  lastName: last,
  password: newToken(),
  picture: picture === "" ? null : picture,
  role: "USER",
  userType: "COMPANY",
  language: "en",
  googleId: googleID,
  githubId: githubID,
  isActive: true,
  isBlocked: false,
  isVerified: true,
});

// googleNames applies the reference defaults: given/family fall back to
// "User"/"Google" when the profile omits them.
const googleNames = (id: GoogleIdentity): [string, string] => {
  const first = id.givenName.trim() === "" ? "User" : id.givenName.trim();
  const last = id.familyName.trim() === "" ? "Google" : id.familyName.trim();
  return [first, last];
};

// githubNames splits the display name (login fallback): first name = first
// word, last name = rest or "GitHub".
const githubNames = (gh: GitHubUser): [string, string] => {
  let name = gh.name;
  if (name === "") {
    name = gh.login;
  }
  const parts = name.trim().split(" ", 2);
  const first = (parts[0] ?? "").trim() === "" ? "User" : (parts[0] ?? "").trim();
  let last = "GitHub";
  if (parts.length > 1 && (parts[1] ?? "").trim() !== "") {
    last = (parts[1] ?? "").trim();
  }
  return [first, last];
};

// isNoreply reports a GitHub noreply placeholder address.
const isNoreply = (email: string): boolean => email.includes("noreply.github.com");

// frontendBase is the registered frontend success URL (trailing slash trimmed).
const frontendBase = (frontend: string): string => frontend.replace(/\/+$/, "");

// linkSuccessURL sends a completed link flow back to the profile page.
const linkSuccessURL = (frontend: string): string =>
  frontendBase(frontend).replace(/\/auth\/success$/, "") + linkSuccessRedirect;

export { frontendBase, linkSuccessURL };
