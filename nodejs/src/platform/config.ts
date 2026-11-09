// Platform config loads typed configuration from the environment (zod-validated)
// and enforces secret strength. The <app>_ env prefix is substituted at
// instantiation time by scripts/rename.sh. Port of the Go template's config
// package: same variables, same defaults, same fail-closed rules.

import { z } from "zod";

// Duration strings ("15m" | "720h" | "2s" | "1h") parsed to milliseconds.
const durationMs = z
  .string()
  .regex(/^\d+[smh]$/, "must be like 15m / 2s / 720h")
  .transform((s) => {
    const n = Number.parseInt(s.slice(0, -1), 10);
    const u = s.slice(-1);
    return u === "h" ? n * 3_600_000 : u === "m" ? n * 60_000 : n * 1000;
  });

const bool = z.enum(["true", "false"]).transform((v) => v === "true");
const int = z.coerce.number().int();
// Comma-separated list -> string[] (exact items; empty string -> []).
const csv = z.string().transform((s) =>
  s
    .split(",")
    .map((x) => x.trim())
    .filter((x) => x !== ""),
);

// The raw shape mirrors the Go template's Config struct field-for-field; the
// schema only carries the non-defaulted fields.
const envSchema = z.object({
  databaseUrl: z.string().min(1),
  jwtSecret: z.string(),
  pepper: z.string(),
  corsOrigins: csv,
  googleClientId: z.string(),
  githubClientId: z.string(),
  githubSecret: z.string(),
  githubCallback: z.string().min(1),
  frontendSuccess: z.string().min(1),
});

export interface Config {
  // Runtime
  env: string;
  bind: string;
  logLevel: string;
  corsOrigins: string[];

  // Postgres
  databaseUrl: string;
  dbMaxConns: number;

  // Auth / security
  jwtSecret: string;
  pepper: string; // optional; empty = disabled
  accessTokenTtlMs: number;
  refreshTokenTtlMs: number;
  verifyCodeTtlMs: number;
  resetCodeTtlMs: number;
  authCookie: boolean; // also set cookie "<app>_access_token"
  resetRateLimit: number;
  resetRateWindowMs: number;
  maxLoginRetries: number;
  logoutAllOnReset: boolean;

  // OAuth sign-in (reference parity). Google verifies the browser's One Tap ID
  // token (empty client id = unconfigured, endpoint answers 401). GitHub
  // ships disabled (reference default): both github routes answer 403 until
  // enabled with id + secret. Both flows perform outbound sockets, so they
  // refuse with a config-style error while <APP>_OFFLINE=true (INV-T7).
  googleClientId: string;
  googleEnabled: boolean;
  githubClientId: string;
  githubSecret: string;
  githubEnabled: boolean;
  githubCallback: string;
  frontendSuccess: string;

  // Janitor: worker-loop purge of expired sessions and stale codes (1h).
  janitorPollMs: number;

  // Bootstrap whitelist for the unauthenticated POST /auth/upgrade-to-admin
  // route. Comma-separated emails, matched case-insensitively; empty
  // disables the route entirely.
  allowedAdminEmails: string[];

  // SMTP (empty => console sender; <APP>_OFFLINE=true forces console, INV-T7)
  smtpHost: string;
  smtpPort: number;
  smtpUser: string;
  smtpPass: string;
  smtpFrom: string;

  // INV-T7 offline guard: true => console mailer, no sockets opened.
  offline: boolean;

  // Email outbox worker
  outboxPollMs: number;
  outboxSize: number;
}

const env = (key: string): string | undefined => process.env[`<APP>_${key}`];
const d = (key: string, fallback: string): string => env(key) ?? fallback;

export function load(): Config {
  const raw = {
    // Runtime
    env: d("ENV", "development"),
    bind: d("BIND", "0.0.0.0:8080"),
    logLevel: d("LOG_LEVEL", "info"),

    // Postgres
    databaseUrl: env("DATABASE_URL") ?? "",
    dbMaxConns: d("DATABASE_MAX_CONNECTIONS", "20"),

    // Auth / security
    jwtSecret: env("JWT_SECRET") ?? "",
    pepper: env("PASSWORD_PEPPER") ?? "",
    corsOrigins: env("CORS_ORIGINS") ?? "http://localhost:5173",
    accessTokenTtl: d("ACCESS_TOKEN_TTL", "15m"),
    refreshTokenTtl: d("REFRESH_TOKEN_TTL", "720h"),
    verifyCodeTtl: d("VERIFY_CODE_TTL", "24h"),
    resetCodeTtl: d("RESET_CODE_TTL", "30m"),
    authCookie: d("AUTH_COOKIE", "false"),
    resetRateLimit: d("RESET_RATE_LIMIT", "3"),
    resetRateWindow: d("RESET_RATE_WINDOW", "5m"),
    maxLoginRetries: d("MAX_LOGIN_RETRIES", "5"),
    logoutAllOnReset: d("LOGOUT_ALL_ON_RESET", "true"),

    // OAuth
    googleClientId: env("GOOGLE_CLIENT_ID") ?? "",
    googleEnabled: d("GOOGLE_ENABLED", "true"),
    githubClientId: env("GITHUB_CLIENT_ID") ?? "",
    githubSecret: env("GITHUB_CLIENT_SECRET") ?? "",
    githubEnabled: d("GITHUB_ENABLED", "false"),
    githubCallback: d(
      "AUTH_GITHUB_CALLBACK_URL",
      "http://localhost:8080/api/v1/auth/github/callback",
    ),
    frontendSuccess: d("AUTH_FRONTEND_SUCCESS", "http://localhost:5173/auth/success"),

    // Janitor
    janitorPoll: d("JANITOR_POLL", "1h"),

    // Bootstrap admin whitelist
    allowedAdminEmails: env("ALLOWED_ADMIN_EMAILS") ?? "",

    // SMTP
    smtpHost: env("SMTP_HOST") ?? "",
    smtpPort: d("SMTP_PORT", "587"),
    smtpUser: env("SMTP_USER") ?? "",
    smtpPass: env("SMTP_PASS") ?? "",
    smtpFrom: env("SMTP_FROM") ?? "",

    // INV-T7 + outbox
    offline: d("OFFLINE", "false"),
    outboxPoll: d("OUTBOX_POLL", "2s"),
    outboxSize: d("OUTBOX_BATCH", "10"),
  };

  // CORS origins keep the leading-default CSV semantics (exact allowlist,
  // "*" matches everything in development only).
  const parsed = envSchema
    .extend({
      env: z.string().min(1),
      bind: z.string().min(1),
      logLevel: z.enum(["debug", "info", "warn", "error"]),
      dbMaxConns: int.pipe(z.number().min(1)),
      accessTokenTtl: durationMs,
      refreshTokenTtl: durationMs,
      verifyCodeTtl: durationMs,
      resetCodeTtl: durationMs,
      authCookie: bool,
      resetRateLimit: int.pipe(z.number().min(0)),
      resetRateWindow: durationMs,
      maxLoginRetries: int.pipe(z.number().min(0)),
      logoutAllOnReset: bool,
      googleEnabled: bool,
      githubEnabled: bool,
      janitorPoll: durationMs,
      allowedAdminEmails: csv,
      smtpPort: int,
      smtpFrom: z.string(),
      smtpHost: z.string(),
      smtpUser: z.string(),
      smtpPass: z.string(),
      offline: bool,
      outboxPoll: durationMs,
      outboxSize: int.pipe(z.number().min(1)),
    })
    .safeParse(raw);

  if (!parsed.success) {
    const i0 = parsed.error.issues[0];
    const where = i0 ? `${i0.path.join(".") || "config"}` : "config";
    throw new Error(
      `config: <APP>_-prefixed env is invalid — ${where}: ${i0?.message ?? "parse error"}`,
    );
  }

  const v = parsed.data;

  // Secret rules (fail closed): the server REFUSES placeholder/default values.
  if (
    v.jwtSecret.length < 32 ||
    v.jwtSecret.includes("<set-me") ||
    v.jwtSecret.includes("change-me-32-bytes")
  ) {
    throw new Error(
      "config: <APP>_JWT_SECRET must be 32+ random bytes (openssl rand -base64 32); placeholder/default rejected",
    );
  }
  if (v.pepper !== "" && v.pepper.length < 32) {
    throw new Error("config: <APP>_PASSWORD_PEPPER, when set, must be 32+ bytes");
  }
  if (v.offline && (v.smtpHost !== "" || v.smtpUser !== "" || v.smtpPass !== "")) {
    throw new Error(
      "config: <APP>_OFFLINE=true forbids SMTP credentials (INV-T7); use the console sender",
    );
  }

  return {
    env: v.env,
    bind: v.bind,
    logLevel: v.logLevel,
    corsOrigins: v.corsOrigins,
    databaseUrl: v.databaseUrl,
    dbMaxConns: v.dbMaxConns,
    jwtSecret: v.jwtSecret,
    pepper: v.pepper,
    accessTokenTtlMs: v.accessTokenTtl,
    refreshTokenTtlMs: v.refreshTokenTtl,
    verifyCodeTtlMs: v.verifyCodeTtl,
    resetCodeTtlMs: v.resetCodeTtl,
    authCookie: v.authCookie,
    resetRateLimit: v.resetRateLimit,
    resetRateWindowMs: v.resetRateWindow,
    maxLoginRetries: v.maxLoginRetries,
    logoutAllOnReset: v.logoutAllOnReset,
    googleClientId: v.googleClientId,
    googleEnabled: v.googleEnabled,
    githubClientId: v.githubClientId,
    githubSecret: v.githubSecret,
    githubEnabled: v.githubEnabled,
    githubCallback: v.githubCallback,
    frontendSuccess: v.frontendSuccess,
    janitorPollMs: v.janitorPoll,
    allowedAdminEmails: v.allowedAdminEmails,
    smtpHost: v.smtpHost,
    smtpPort: v.smtpPort,
    smtpUser: v.smtpUser,
    smtpPass: v.smtpPass,
    smtpFrom: v.smtpFrom,
    offline: v.offline,
    outboxPollMs: v.outboxPoll,
    outboxSize: v.outboxSize,
  };
}
