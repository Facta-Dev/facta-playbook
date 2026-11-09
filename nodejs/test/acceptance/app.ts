// Acceptance app: assembles the real HTTP app exactly like bin/api.ts — the
// same register calls over in-memory fakes, a recorded email enqueuer, a
// fixed clock and an app-local hub, listening on an ephemeral port. ponytail:
// healthz/readyz and the request-id/security-headers/CORS chain are not
// assembled; no feature exercises those.

import cookie from "@fastify/cookie";
import { fastify } from "fastify";
import { maxBodyBytes, sendJSON, envelope } from "#platform/httpx.js";
import { newHub } from "#platform/realtime/index.js";
import { LinkToken } from "#platform/oauthx.js";
import { E, NotFound, InternalError, Validation, AppError } from "#shared/apperr.js";
import { registerWidgets } from "#features/widgets/module.js";
import { registerAuthsignup } from "#features/authsignup/module.js";
import { registerAuthsession } from "#features/authsession/module.js";
import { registerAuthoauth } from "#features/authoauth/module.js";
import { registerAuthpassword } from "#features/authpassword/module.js";
import { registerAdminusers } from "#features/adminusers/module.js";
import {
  AdminStore,
  FakeGitHub,
  FakeGoogle,
  UserTable,
  SignupStore,
  SessionStore,
  OauthStore,
  PasswordStore,
  WidgetStore,
} from "./fakes.js";
import {
  RecordedEmails,
  TestClock,
  newMinter,
  newVerifier,
  TestJWTSecret,
  TestPepper,
} from "../testutil.js";

// TTLs of the flows under test; mirror the config defaults named in the
// features (24h verify, 15m access, 720h refresh, 30m reset).
export const VerifyTTL = 24 * 60 * 60_000;
export const AccessTTL = 15 * 60_000;
export const RefreshTTL = 720 * 60 * 60_000;
export const ResetTTL = 30 * 60_000;

// OAuthFlags are the social-provider switches the toggle steps flip by
// rebuilding the app (the use case copies its config at registration).
export interface OAuthFlags {
  google: boolean;
  github: boolean;
}

// AppHandle is the live app the steps drive.
export interface AppHandle {
  base: string;
  table: UserTable;
  emails: RecordedEmails;
  clock: TestClock;
  hub: ReturnType<typeof newHub>;
  stop(): Promise<void>;
}

// newAppWith assembles the real HTTP app exactly like bin/api.ts — the same
// register calls, with in-memory fakes and an app-local hub.
export const newAppWith = async (flags: OAuthFlags): Promise<AppHandle> => {
  const clock = new TestClock();
  const table = new UserTable(clock);
  const emails = new RecordedEmails();
  const verifier = newVerifier();
  const minter = newMinter();
  const hub = newHub(16);

  const app = fastify({ logger: false, bodyLimit: maxBodyBytes });
  await app.register(cookie);
  // fallback: any unmatched path is the JSON 404 envelope (bin/api.ts parity;
  // the httpx error mapping rides the AppError-aware handler below).
  app.setNotFoundHandler((_req, reply) => {
    sendJSON(reply, 404, envelope(E(404, NotFound, "route not found")));
  });
  app.setErrorHandler((err, _req, reply) => {
    const meta = err as { statusCode?: number; message?: string };
    const status = typeof meta.statusCode === "number" ? meta.statusCode : 500;
    const ae: AppError =
      err instanceof AppError
        ? err
        : status === 413
          ? E(400, Validation, "invalid JSON body")
          : status < 500
            ? E(status, Validation, meta.message ?? "request failed")
            : E(500, InternalError, "internal server error");
    sendJSON(reply, ae.http, envelope(ae));
  });

  registerWidgets(app, {
    store: new WidgetStore(table),
    publisher: hub,
    verifier,
    cookieName: "",
  });

  registerAuthsignup(app, {
    store: new SignupStore(table),
    emails,
    clock,
    pepper: TestPepper,
    verifyTtlMs: VerifyTTL,
  });

  registerAuthsession(app, {
    store: new SessionStore(table),
    minter,
    verifier,
    clock,
    pepper: TestPepper,
    cookieName: "",
    cookieSecure: false,
    accessTtlMs: AccessTTL,
    refreshTtlMs: RefreshTTL,
    maxLoginRetry: 5,
  });

  registerAuthoauth(app, {
    store: new OauthStore(table),
    google: new FakeGoogle(),
    github: new FakeGitHub(),
    states: new LinkToken(TestJWTSecret),
    minter,
    verifier,
    clock,
    googleClientID: "test-google-client-id",
    googleEnabled: flags.google,
    githubClientID: "test-github-client-id",
    githubClientSecret: "test-github-secret",
    githubCallbackURL: "http://localhost:8080/api/v1/auth/github/callback",
    githubEnabled: flags.github,
    frontendSuccess: "http://localhost:5173/auth/success",
    offline: false,
    cookieName: "",
    cookieSecure: false,
    accessTtlMs: AccessTTL,
    refreshTtlMs: RefreshTTL,
  });

  registerAuthpassword(app, {
    store: new PasswordStore(table),
    emails,
    clock,
    verifier,
    pepper: TestPepper,
    cookieName: "",
    resetTtlMs: ResetTTL,
    rateLimit: 3,
    rateWindowMs: 5 * 60_000,
    logOutAllOnReset: true,
  });

  registerAdminusers(app, {
    store: new AdminStore(table),
    verifier,
    cookieName: "",
    whitelist: ["admin-boost@ex.com"],
  });

  // start: listen on an ephemeral port (fetch hits the real HTTP stack; SSE
  // needs a true server, not app.inject).
  await app.listen({ port: 0, host: "127.0.0.1" });
  const addr = app.server.address();
  const port = typeof addr === "object" && addr !== null ? addr.port : 0;
  return {
    base: `http://127.0.0.1:${port}`,
    table,
    emails,
    clock,
    hub,
    stop: (): Promise<void> => app.close(),
  };
};
