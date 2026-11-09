// Features authoauth ports: consumer-declared interfaces. oauthx satisfies
// the provider ports; platform/authx satisfies TokenMinter; postgres.ts
// implements Store.

import type { GoogleIdentity, GitHubUser } from "#platform/oauthx.js";
import type { Identity } from "#platform/authx.js";
import type { User, SessionRow } from "./domain.js";

// GoogleVerifier verifies One-Tap credentials (impl: oauthx, RS256 over the
// cached Google JWKS).
export interface GoogleVerifier {
  verify(credential: string, clientID: string): Promise<GoogleIdentity>;
}

// GitHubClient builds the authorize URL and exchanges codes (impl: oauthx).
export interface GitHubClient {
  authorize(clientID: string, redirectURI: string, state: string): string;
  exchange(
    code: string,
    clientID: string,
    clientSecret: string,
    redirectURI: string,
  ): Promise<GitHubUser>;
}

// LinkState mints/parses the ?state= link JWT ({sub, purpose:github_link};
// impl: oauthx.LinkToken over the shared HS256 secret).
export interface LinkState {
  mintLink(userID: string): Promise<string>;
  parseLink(state: string): Promise<string>;
}

// TokenMinter port on the shared authx (platform/authx satisfies it).
export interface TokenMinter {
  mint(id: Identity, ttlMs: number): Promise<{ token: string; exp: Date }>;
}

// Store persists users + sessions for the social flows (impl: postgres.ts).
export interface Store {
  getUserByGoogleID(googleID: string): Promise<User | null>;
  getUserByGithubID(githubID: string): Promise<User | null>;
  getUserByEmail(email: string): Promise<User | null>;
  getUserByID(id: string): Promise<User | null>;
  // createUser provisions the social user (auto-verified, unusable password).
  createUser(u: User): Promise<void>;
  // linkGoogle/linkGithub attach the provider id to an email-matched user
  // (verifying it and filling the picture when empty, SQL-side).
  linkGoogle(userID: string, providerID: string, picture: string): Promise<void>;
  linkGithub(userID: string, providerID: string, picture: string): Promise<void>;
  // githubOnly is the link-flow write: attach the github id, nothing else.
  githubOnly(userID: string, githubID: string): Promise<void>;
  // saveGithubLogin applies the login-flow identity refresh (email upgrade
  // decided in the use case; picture-if-empty and verified mark SQL-side).
  saveGithubLogin(userID: string, email: string, picture: string): Promise<void>;
  createSession(s: SessionRow): Promise<void>;
}
