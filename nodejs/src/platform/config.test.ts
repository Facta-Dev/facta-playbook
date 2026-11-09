// Config rules are fail-closed: placeholder/weak secrets never boot, and the
// INV-T7 offline guard forbids SMTP credentials.

import { afterEach, describe, expect, it } from "vitest";
import { load } from "#platform/config.js";

const VALID_SECRET = "0123456789abcdef0123456789abcdef"; // 32 chars

const withEnv = (vars: Record<string, string>, fn: () => void): void => {
  const keep = { ...process.env };
  for (const [k, v] of Object.entries(vars)) process.env[`<APP>_${k}`] = v;
  try {
    fn();
  } finally {
    process.env = keep;
  }
};

const base = { JWT_SECRET: VALID_SECRET, DATABASE_URL: "postgres://localhost:5432/x" };

describe("config.load", () => {
  afterEach(() => {
    for (const k of Object.keys(process.env)) {
      if (k.startsWith("<app>_")) delete process.env[k];
    }
  });

  it("rejects a short JWT secret", () => {
    withEnv({ ...base, JWT_SECRET: "short" }, () => {
      expect(() => load()).toThrow(/JWT_SECRET/);
    });
  });

  it("rejects placeholder secrets", () => {
    withEnv({ ...base, JWT_SECRET: "change-me-32-bytes-min-rotate-me" }, () => {
      expect(() => load()).toThrow(/placeholder/);
    });
    withEnv({ ...base, JWT_SECRET: "<set-me-32-bytes-random-please!!" }, () => {
      expect(() => load()).toThrow(/placeholder/);
    });
  });

  it("rejects a short pepper when set", () => {
    withEnv({ ...base, PASSWORD_PEPPER: "tooshort" }, () => {
      expect(() => load()).toThrow(/PASSWORD_PEPPER/);
    });
  });

  it("rejects SMTP credentials while OFFLINE=true (INV-T7)", () => {
    withEnv({ ...base, OFFLINE: "true", SMTP_HOST: "smtp.example.invalid" }, () => {
      expect(() => load()).toThrow(/console sender/);
    });
  });

  it("loads with defaults and parses durations", () => {
    withEnv(base, () => {
      const cfg = load();
      expect(cfg.env).toBe("development");
      expect(cfg.bind).toBe("0.0.0.0:8080");
      expect(cfg.accessTokenTtlMs).toBe(15 * 60_000);
      expect(cfg.refreshTokenTtlMs).toBe(720 * 3_600_000);
      expect(cfg.offline).toBe(false);
      expect(cfg.corsOrigins).toEqual(["http://localhost:5173"]);
    });
  });

  it("parses CSV lists and booleans", () => {
    withEnv(
      { ...base, ALLOWED_ADMIN_EMAILS: "A@x.io, b@y.io ,", LOGOUT_ALL_ON_RESET: "true" },
      () => {
        const cfg = load();
        expect(cfg.allowedAdminEmails).toEqual(["A@x.io", "b@y.io"]);
        expect(cfg.logoutAllOnReset).toBe(true);
      },
    );
  });
});
