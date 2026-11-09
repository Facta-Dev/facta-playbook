// The JWT roundtrip is the core contract: minted tokens verify, tampered or
// expired ones do not, and only HS256 is accepted.

import { describe, expect, it } from "vitest";
import { newHS256Minter, newHS256Verifier } from "#platform/authx.js";

const SECRET = "0123456789abcdef0123456789abcdef";
const identity = { userId: "u-1", email: "a@x.io", role: "USER" as const, jti: "j-1" };

describe("authx HS256", () => {
  it("roundtrips mint -> parse", async () => {
    const minter = newHS256Minter(SECRET, "<app>");
    const verifier = newHS256Verifier(SECRET, "<app>");
    const { token, exp } = await minter.mint(identity, 60_000);
    expect(exp.getTime()).toBeGreaterThan(Date.now());
    const parsed = await verifier.parse(token);
    expect(parsed).toEqual(identity);
  });

  it("refuses tampered tokens with AUTH_INVALID_TOKEN", async () => {
    const minter = newHS256Minter(SECRET, "<app>");
    const verifier = newHS256Verifier(SECRET, "<app>");
    const { token } = await minter.mint(identity, 60_000);
    const parts = token.split(".");
    await expect(
      verifier.parse(`${parts[0]}.AAAA${(parts[1] ?? "").slice(4)}.${parts[2]}`),
    ).rejects.toMatchObject({
      code: "AUTH_INVALID_TOKEN",
      http: 401,
    });
  });

  it("checks the issuer and expiry", async () => {
    const minter = newHS256Minter(SECRET, "<app>");
    const verifier = newHS256Verifier(SECRET, "other-issuer");
    const { token } = await minter.mint(identity, 60_000);
    await expect(verifier.parse(token)).rejects.toBeTruthy();

    const exp0 = newHS256Minter(SECRET, "<app>");
    const v0 = newHS256Verifier(SECRET, "<app>");
    const expired = await exp0.mint(identity, -1_000);
    await expect(v0.parse(expired.token)).rejects.toMatchObject({ http: 401 });
  });
});
