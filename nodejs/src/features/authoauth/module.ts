// Features authoauth module: the slice's registration seam. bin/api.ts is
// the only caller — slices never see each other. The google endpoint answers
// the JSON envelope; the github endpoints ALWAYS 302 (or the 403 envelope
// when the provider is unconfigured).

import type { FastifyInstance } from "fastify";
import { requireAuth, type Verifier } from "#platform/authx.js";
import type { Clock } from "#shared/clockx.js";
import { authoauthHttp } from "./http.js";
import { UseCase } from "./usecase.js";
import type { GoogleVerifier, GitHubClient, LinkState, TokenMinter, Store } from "./ports.js";

// Deps carries everything register needs; bin/api.ts is the only caller.
export interface Deps {
  store: Store;
  google: GoogleVerifier; // oauthx
  github: GitHubClient; // oauthx
  states: LinkState; // oauthx LinkToken
  minter: TokenMinter;
  verifier: Verifier; // guards connect-url
  clock: Clock;

  googleClientID: string;
  googleEnabled: boolean;
  githubClientID: string;
  githubClientSecret: string;
  githubCallbackURL: string; // <APP>_AUTH_GITHUB_CALLBACK_URL
  githubEnabled: boolean;
  frontendSuccess: string; // <APP>_AUTH_FRONTEND_SUCCESS
  offline: boolean; // INV-T7: refuse before any socket

  cookieName: string; // "" disables the cookie fallback; forwarded to requireAuth
  cookieSecure: boolean; // Set-Cookie Secure flag (false only for dev)
  accessTtlMs: number;
  refreshTtlMs: number;
}

// register mounts the social-login routes. connect-url sits behind requireAuth
// (a link state is a bearer-bound secret; reference parity — no role gate).
export const registerAuthoauth = (app: FastifyInstance, d: Deps): void => {
  const uc = new UseCase(d.store, d.google, d.github, d.states, d.minter, d.clock, {
    googleClientID: d.googleClientID,
    googleEnabled: d.googleEnabled,
    githubClientID: d.githubClientID,
    githubClientSecret: d.githubClientSecret,
    githubCallbackURL: d.githubCallbackURL,
    githubEnabled: d.githubEnabled,
    frontendSuccess: d.frontendSuccess,
    offline: d.offline,
    accessTtlMs: d.accessTtlMs,
    refreshTtlMs: d.refreshTtlMs,
  });
  const h = authoauthHttp(uc, {
    accessTtlMs: d.accessTtlMs,
    cookieName: d.cookieName,
    cookieSecure: d.cookieSecure,
    frontendSuccess: d.frontendSuccess,
  });
  app.post("/api/v1/auth/google", h.google);
  app.get("/api/v1/auth/github/login", h.githubLogin);
  app.get("/api/v1/auth/github/callback", h.githubCallback);
  app.get(
    "/api/v1/auth/github/connect-url",
    { preHandler: requireAuth(d.verifier, d.cookieName) },
    h.githubConnect,
  );
};
