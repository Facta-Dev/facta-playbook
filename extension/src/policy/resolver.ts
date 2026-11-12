// <app> — the effective policy.
//
// PLAYBOOK §7.3: the administrator policy wins, the user settings fill the
// gaps, the code defaults come last. The resolver is pure, so the
// precedence can be checked without a browser.

import {
  type LockedSetting,
  type ManagedPolicy,
  type PolicySettings,
  type UserPolicy,
} from "../contracts/policy";
import { matchesPattern } from "../security/matchers";

/** The values the extension falls back to. */
export const DEFAULT_POLICY: PolicySettings = {
  allowedHosts: [],
  blockedHosts: [],
  featureFlags: {},
  killSwitch: false,
  allowUserSettings: true,
};

export interface EffectivePolicy {
  readonly settings: PolicySettings;
  /** The settings the administrator locked; a UI renders them read-only. */
  readonly locked: readonly LockedSetting[];
}

export function resolvePolicy(managed: ManagedPolicy, user: UserPolicy): EffectivePolicy {
  const locked: LockedSetting[] = [];
  if (managed.AllowedHosts !== undefined) locked.push("allowedHosts");
  if (managed.BlockedHosts !== undefined) locked.push("blockedHosts");
  if (managed.FeatureFlags !== undefined) locked.push("featureFlags");
  if (managed.KillSwitch !== undefined) locked.push("killSwitch");
  if (managed.AllowUserSettings !== undefined) locked.push("allowUserSettings");

  const settings: PolicySettings = {
    allowedHosts: managed.AllowedHosts ?? user.AllowedHosts ?? DEFAULT_POLICY.allowedHosts,
    blockedHosts: managed.BlockedHosts ?? user.BlockedHosts ?? DEFAULT_POLICY.blockedHosts,
    // A flag set by the administrator wins; the user keeps the other flags.
    featureFlags: { ...user.FeatureFlags, ...managed.FeatureFlags },
    killSwitch: managed.KillSwitch ?? user.KillSwitch ?? DEFAULT_POLICY.killSwitch,
    allowUserSettings:
      managed.AllowUserSettings ?? user.AllowUserSettings ?? DEFAULT_POLICY.allowUserSettings,
  };

  return { settings, locked };
}

/**
 * True when the extension may run on the url. The administrator blocks
 * first, and an empty allowlist allows nothing: an unknown policy is the
 * most restrictive one.
 */
export function isUrlAllowed(settings: PolicySettings, url: string): boolean {
  if (settings.killSwitch) return false;
  if (settings.blockedHosts.some((pattern) => matchesPattern(pattern, url))) return false;
  return settings.allowedHosts.some((pattern) => matchesPattern(pattern, url));
}
