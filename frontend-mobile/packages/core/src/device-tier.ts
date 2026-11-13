// <app> — the device tier.
//
// The tier is the app's guess about the hardware it runs on. It decides the
// quality profile, the render scale and the frame budget, so it has to be
// conservative on the low end: a phone that is too slow is a crash, a phone
// that is faster than advertised is only a bit of wasted headroom.
//
// Sources: total RAM, the low-RAM flag, the OS/API level and the measured
// frame time of the first-run micro-benchmark. The measurement can only
// lower or confirm the hint — never raise it (PLAYBOOK_FRONTEND_MOBILE.md
// §3.3).

/** The three device classes the budgets are written against. */
export const DEVICE_TIERS = ["low", "mid", "high"] as const;

/** The device class of the current phone. */
export type DeviceTier = (typeof DEVICE_TIERS)[number];

/** The signals the tier is derived from. Every field is optional: the
 *  caller passes what it managed to read. */
export interface DeviceSignals {
  /** Total RAM in gigabytes. */
  readonly totalRamGb?: number;
  /** The OS low-RAM flag (Android `isLowRamDevice`). */
  readonly lowRamDevice?: boolean;
  /** The OS / API level. */
  readonly osApiLevel?: number;
  /** The measured p95 frame time of the reference scene, in milliseconds. */
  readonly measuredFrameP95Ms?: number;
}

/** The p95 frame time above which a device is treated as low tier. */
export const LOW_TIER_FRAME_P95_MS = 33;

/** The p95 frame time above which a device is capped at mid tier. */
export const MID_TIER_FRAME_P95_MS = 20;

/** The oldest API level the app supports. */
export const MIN_OS_API_LEVEL = 26;

/**
 * Parses a tier out of a string that came from a remote config or a
 * persisted preference. Returns null for anything unknown: a broken
 * override falls back to the detected tier, it never guesses.
 */
export function parseDeviceTier(value: string): DeviceTier | null {
  const normalized = value.trim().toLowerCase();
  return (DEVICE_TIERS as readonly string[]).includes(normalized)
    ? (normalized as DeviceTier)
    : null;
}

/** The lower of two tiers — the conservative answer. */
export function weakerTier(a: DeviceTier, b: DeviceTier): DeviceTier {
  return DEVICE_TIERS.indexOf(a) <= DEVICE_TIERS.indexOf(b) ? a : b;
}

/**
 * The tier the hardware hints point at.
 *
 * The rules, in order: the low-RAM flag wins, then the RAM size, then the
 * OS level. A device with no signal at all is mid tier: it is the market
 * default, and the measurement corrects it.
 */
export function tierFromSignals(signals: DeviceSignals): DeviceTier {
  if (signals.lowRamDevice === true) return "low";

  const ram = signals.totalRamGb;
  if (ram !== undefined) {
    if (ram <= 3) return "low";
    if (ram <= 6) return "mid";
    return "high";
  }

  const api = signals.osApiLevel;
  if (api !== undefined) {
    if (api < MIN_OS_API_LEVEL) return "low";
    if (api <= 30) return "mid";
    return "high";
  }

  return "mid";
}

/**
 * The final tier: the hardware hint, lowered by the measurement.
 *
 * The measurement never raises the hint — a phone that once rendered one
 * scene fast is not evidence that it is a high-tier phone.
 */
export function resolveDeviceTier(signals: DeviceSignals): DeviceTier {
  const hint = tierFromSignals(signals);
  const measured = signals.measuredFrameP95Ms;
  if (measured === undefined) return hint;
  if (measured > LOW_TIER_FRAME_P95_MS) return "low";
  if (measured > MID_TIER_FRAME_P95_MS) return weakerTier(hint, "mid");
  return hint;
}
