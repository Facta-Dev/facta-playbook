// Command api is the composition root: config → pool → ports → slices →
// middleware → HTTP server. Everything wires here and nowhere else; slices
// never see each other.

import cookie from "@fastify/cookie";
import { fastify } from "fastify";
import { newHS256Verifier, newHS256Minter } from "#platform/authx.js";
import { load } from "#platform/config.js";
import { maxBodyBytes, sendJSON, envelope } from "#platform/httpx.js";
import { newPool, readyz } from "#platform/pg.js";
import { GoogleVerifier, GitHubClient, LinkToken } from "#platform/oauthx.js";
import { newHub } from "#platform/realtime/index.js";
import { Outbox, selectSender, runEmailWorker } from "#platform/email/index.js";
import { E, NotFound, InternalError, Validation, AppError } from "#shared/apperr.js";
import { systemClock } from "#shared/clockx.js";
import { registerWidgets } from "#features/widgets/module.js";
import { registerRealtime } from "#features/realtime/module.js";
import { Postgres as WidgetsPostgres } from "#features/widgets/postgres.js";
import { registerAuthsignup } from "#features/authsignup/module.js";
import { Postgres as SignupPostgres } from "#features/authsignup/postgres.js";
import { registerAuthsession } from "#features/authsession/module.js";
import { Postgres as SessionPostgres } from "#features/authsession/postgres.js";
import { registerAuthoauth } from "#features/authoauth/module.js";
import { Postgres as OauthPostgres } from "#features/authoauth/postgres.js";
import { registerAuthpassword } from "#features/authpassword/module.js";
import { Postgres as PasswordPostgres } from "#features/authpassword/postgres.js";
import { registerAdminusers } from "#features/adminusers/module.js";
import { Postgres as AdminPostgres } from "#features/adminusers/postgres.js";
import { cors, requestID, securityHeaders } from "./middleware.js";

const main = async (): Promise<void> => {
  const cfg = load();

  // probe: container healthcheck — the runtime image carries no shell/curl,
  // so the api probes its own /healthz and exits non-zero on failure.
  if (process.argv[2] === "probe") {
    await probeHealth(cfg.bind);
    return;
  }

  const app = fastify({ logger: { level: cfg.logLevel }, bodyLimit: maxBodyBytes });
  await app.register(cookie);

  // chain: request-id, security headers, CORS (cmd/api/middleware.go parity).
  app.addHook("onRequest", requestID);
  app.addHook("onRequest", securityHeaders);
  app.addHook("onRequest", cors(cfg));

  // fallback: any unmatched path is the JSON 404 envelope.
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
    if (ae === err) {
      app.log.warn({ code: ae.code }, "request error");
    } else {
      app.log.error(err);
    }
    sendJSON(reply, ae.http, envelope(ae));
  });

  const pool = await newPool(cfg);
  const verifier = newHS256Verifier(cfg.jwtSecret, "<app>");
  const minter = newHS256Minter(cfg.jwtSecret, "<app>");
  const hub = newHub(0); // hub = per-subscriber buffer, hub default 16 (batch size is the outbox fetch)
  const cookieName = cfg.authCookie ? "<app>_access_token" : "";
  const cookieSecure = cfg.env !== "dev";
  // health mounts outside auth; readyz degrades to a 503 envelope when
  // the database stops answering.
  app.get("/healthz", async () => ({ status: "ok" }));
  app.get("/readyz", async (_req, reply) => {
    try {
      await readyz(pool, 2000);
      return { status: "ready" };
    } catch {
      return sendJSON(reply, 503, envelope(E(503, InternalError, "database not ready")));
    }
  });

  registerWidgets(app, {
    store: new WidgetsPostgres(pool),
    publisher: hub,
    verifier,
    cookieName,
  });

  registerRealtime(app, { verifier, publisher: hub });

  registerAuthsignup(app, {
    store: new SignupPostgres(pool),
    emails: new Outbox(pool),
    clock: systemClock,
    pepper: cfg.pepper,
    verifyTtlMs: cfg.verifyCodeTtlMs,
  });

  registerAuthsession(app, {
    store: new SessionPostgres(pool),
    minter,
    verifier,
    clock: systemClock,
    pepper: cfg.pepper,
    cookieName,
    cookieSecure,
    accessTtlMs: cfg.accessTokenTtlMs,
    refreshTtlMs: cfg.refreshTokenTtlMs,
    maxLoginRetry: cfg.maxLoginRetries,
  });

  registerAuthoauth(app, {
    store: new OauthPostgres(pool),
    google: new GoogleVerifier("", cfg.offline),
    github: new GitHubClient(cfg.offline),
    states: new LinkToken(cfg.jwtSecret),
    minter,
    verifier,
    clock: systemClock,
    googleClientID: cfg.googleClientId,
    googleEnabled: cfg.googleEnabled,
    githubClientID: cfg.githubClientId,
    githubClientSecret: cfg.githubSecret,
    githubCallbackURL: cfg.githubCallback,
    githubEnabled: cfg.githubEnabled,
    frontendSuccess: cfg.frontendSuccess,
    offline: cfg.offline,
    cookieName,
    cookieSecure,
    accessTtlMs: cfg.accessTokenTtlMs,
    refreshTtlMs: cfg.refreshTokenTtlMs,
  });
  registerAuthpassword(app, {
    store: new PasswordPostgres(pool),
    emails: new Outbox(pool),
    clock: systemClock,
    verifier,
    pepper: cfg.pepper,
    cookieName,
    resetTtlMs: cfg.resetCodeTtlMs,
    rateLimit: cfg.resetRateLimit,
    rateWindowMs: cfg.resetRateWindowMs,
    logOutAllOnReset: cfg.logoutAllOnReset,
  });
  registerAdminusers(app, {
    store: new AdminPostgres(pool),
    verifier,
    cookieName,
    whitelist: cfg.allowedAdminEmails,
  });

  // email outbox worker rides the api process (cmd/api/main.go parity); it
  // stops on the same abort that shuts the server down.
  const workerCtl = new AbortController();
  const worker = runEmailWorker(
    new Outbox(pool),
    selectSender(cfg, app.log),
    cfg.outboxPollMs,
    cfg.outboxSize,
    workerCtl.signal,
  );
  worker.catch((err) => app.log.error(err));

  const stop = async (): Promise<void> => {
    workerCtl.abort();
    await worker.catch(() => undefined);
    await app.close();
    await pool.end();
  };
  process.on("SIGTERM", () => {
    void stop();
  });
  process.on("SIGINT", () => {
    void stop();
  });

  await app.listen(parseBind(cfg.bind));
  app.log.info({ addr: cfg.bind }, "listening");
};

// probeHealth GETs http://127.0.0.1:<port>/healthz within 2 seconds (the
// configured bind's host part is ignored — inside the container we always
// hit loopback).
const probeHealth = async (bind: string): Promise<void> => {
  const port = bind.split(":").at(-1) ?? "8080";
  const resp = await fetch(`http://127.0.0.1:${port}/healthz`, {
    signal: AbortSignal.timeout(2000),
  });
  if (!resp.ok) throw new Error(`healthz returned ${resp.status}`);
};

// parseBind maps ":8080" / "0.0.0.0:8080" onto fastify listen options.
const parseBind = (bind: string): { host: string; port: number } => {
  const idx = bind.lastIndexOf(":");
  const host = idx === -1 || bind.slice(0, idx) === "" ? "0.0.0.0" : bind.slice(0, idx);
  const port = Number(bind.slice(idx === -1 ? bind.length : idx + 1));
  return { host, port: Number.isFinite(port) && port > 0 ? port : 8080 };
};

main().catch((err) => {
  console.error("fatal", err);
  process.exit(1);
});
