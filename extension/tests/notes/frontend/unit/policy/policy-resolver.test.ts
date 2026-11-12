// exampletool — the policy precedence.
//
// PLAYBOOK §7.3: the administrator policy wins, the user settings fill the
// gaps, the code defaults come last. The resolver is pure, so the precedence
// is checked without a browser.

import { describe, expect, it } from "vitest";
import { DEFAULT_POLICY, isUrlAllowed, resolvePolicy } from "../../../../../src/policy/resolver";
import { type ManagedPolicy, type UserPolicy } from "../../../../../src/contracts/policy";

describe("policy resolver", () => {
  describe("resolvePolicy", () => {
    it("prefers the administrator policy over the user settings", () => {
      const managed: ManagedPolicy = { AllowedHosts: ["https://admin.example/*"] };
      const user: UserPolicy = { AllowedHosts: ["https://user.example/*"] };

      const policy = resolvePolicy(managed, user);

      expect(policy.settings.allowedHosts).toEqual(["https://admin.example/*"]);
    });

    it("locks the settings the administrator set", () => {
      const managed: ManagedPolicy = {
        AllowedHosts: ["https://admin.example/*"],
        KillSwitch: true,
      };

      const policy = resolvePolicy(managed, {});

      expect(policy.locked).toEqual(["allowedHosts", "killSwitch"]);
    });

    it("fills the gap the administrator left with the user settings", () => {
      const managed: ManagedPolicy = { KillSwitch: true };
      const user: UserPolicy = { AllowedHosts: ["https://user.example/*"] };

      const policy = resolvePolicy(managed, user);

      expect(policy.settings.allowedHosts).toEqual(["https://user.example/*"]);
      expect(policy.settings.killSwitch).toBe(true);
      expect(policy.locked).toEqual(["killSwitch"]);
    });

    it("falls back to the code defaults when nobody set anything", () => {
      const policy = resolvePolicy({}, {});

      expect(policy.settings).toEqual(DEFAULT_POLICY);
      expect(policy.locked).toEqual([]);
    });

    it("keeps the user flags the administrator did not set", () => {
      const managed: ManagedPolicy = { FeatureFlags: { beta: true } };
      const user: UserPolicy = { FeatureFlags: { alpha: false, beta: false } };

      const policy = resolvePolicy(managed, user);

      expect(policy.settings.featureFlags).toEqual({ alpha: false, beta: true });
    });
  });

  describe("isUrlAllowed", () => {
    const settings = {
      allowedHosts: ["https://*.example.com/*"],
      blockedHosts: ["https://blocked.example.com/*"],
      featureFlags: {},
      killSwitch: false,
      allowUserSettings: true,
    };

    it("allows a url that matches the allowlist", () => {
      expect(isUrlAllowed(settings, "https://app.example.com/page")).toBe(true);
    });

    it("refuses a url that matches the blocklist", () => {
      expect(isUrlAllowed(settings, "https://blocked.example.com/page")).toBe(false);
    });

    it("refuses every url when the kill switch is on", () => {
      expect(isUrlAllowed({ ...settings, killSwitch: true }, "https://app.example.com/page")).toBe(
        false,
      );
    });

    it("refuses every url when the allowlist is empty", () => {
      expect(isUrlAllowed({ ...settings, allowedHosts: [] }, "https://app.example.com/page")).toBe(
        false,
      );
    });

    it("refuses a url from another host", () => {
      expect(isUrlAllowed(settings, "https://attacker.example.org/page")).toBe(false);
    });
  });
});
