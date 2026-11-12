// <app> — the policy contract.
//
// Two surfaces: the administrator policy, published through
// chrome.storage.managed, and the user settings the extension stores. Both
// carry the same keys, so the resolver can apply one precedence rule to
// every field. The property names are the Chrome policy keys and must stay
// in sync with managed-schema.json.

import { z } from "zod";

/** A Chrome match pattern, for example "https://*.example.com/*". */
export const HostPatternSchema = z.string().min(1).max(255);

export const FeatureFlagMapSchema = z.record(z.string(), z.boolean());

/**
 * The administrator policy. Unknown keys are dropped instead of failing the
 * whole object: the administrator may publish keys a later build adds.
 */
export const ManagedPolicySchema = z.object({
  AllowedHosts: z.array(HostPatternSchema).optional(),
  BlockedHosts: z.array(HostPatternSchema).optional(),
  FeatureFlags: FeatureFlagMapSchema.optional(),
  KillSwitch: z.boolean().optional(),
  AllowUserSettings: z.boolean().optional(),
});

export type ManagedPolicy = z.infer<typeof ManagedPolicySchema>;

/** The user settings, as the options page writes them. */
export const UserPolicySchema = z.strictObject({
  AllowedHosts: z.array(HostPatternSchema).optional(),
  BlockedHosts: z.array(HostPatternSchema).optional(),
  FeatureFlags: FeatureFlagMapSchema.optional(),
  KillSwitch: z.boolean().optional(),
  AllowUserSettings: z.boolean().optional(),
});

export type UserPolicy = z.infer<typeof UserPolicySchema>;

/** The resolved settings: every field always has a value. */
export const PolicySettingsSchema = z.strictObject({
  allowedHosts: z.array(HostPatternSchema),
  blockedHosts: z.array(HostPatternSchema),
  featureFlags: FeatureFlagMapSchema,
  killSwitch: z.boolean(),
  allowUserSettings: z.boolean(),
});

export type PolicySettings = z.infer<typeof PolicySettingsSchema>;

/** The settings the administrator locked; a UI renders them read-only. */
export const LOCKED_SETTINGS = [
  "allowedHosts",
  "blockedHosts",
  "featureFlags",
  "killSwitch",
  "allowUserSettings",
] as const;

export type LockedSetting = (typeof LOCKED_SETTINGS)[number];
