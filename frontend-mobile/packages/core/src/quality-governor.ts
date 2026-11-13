// <app> — the quality governor.
//
// A pure function: the device tier plus the current sensors in, a quality
// profile out. No timers, no subscriptions, no native calls — the runtime
// loop that calls it (with the hysteresis) lives in the application layer,
// and this function is the part that is worth testing.
//
// The governor only ever steps DOWN from the tier baseline. Stepping up
// again needs a sustained healthy period, and that decision belongs to the
// runtime loop, not here (PLAYBOOK_FRONTEND_MOBILE.md §3.4).

import type { DeviceTier } from "./device-tier";

/** The thermal state reported by the OS. */
export const THERMAL_STATES = ["nominal", "fair", "serious", "critical"] as const;

/** The thermal state of the device. */
export type ThermalState = (typeof THERMAL_STATES)[number];

/** The memory pressure reported by the OS. */
export const MEMORY_PRESSURES = ["normal", "warning", "critical"] as const;

/** The memory pressure of the device. */
export type MemoryPressure = (typeof MEMORY_PRESSURES)[number];

/** What the app renders with. */
export interface QualityProfile {
  /** The render resolution multiplier, 0.5 – 1.0. */
  readonly renderScale: number;
  /** The frame budget in milliseconds: 16 is 60 fps, 33 is 30 fps. */
  readonly frameBudgetMs: number;
  /** Whether shadows and post-processing are drawn. */
  readonly shadowsEnabled: boolean;
  /** The texture resolution cap. */
  readonly textureResolution: number;
  /** The particle budget. */
  readonly particleBudget: number;
}

/** What the governor reads. */
export interface GovernorInput {
  /** The device tier the profile starts from. */
  readonly tier: DeviceTier;
  /** The current thermal state. */
  readonly thermal: ThermalState;
  /** The current memory pressure. */
  readonly memoryPressure: MemoryPressure;
  /** Whether the user or the OS asked for the battery saver. */
  readonly batterySaver?: boolean;
}

/** The profile of a healthy device of the given tier. */
export function baseProfile(tier: DeviceTier): QualityProfile {
  switch (tier) {
    case "low":
      return {
        renderScale: 0.7,
        frameBudgetMs: 33,
        shadowsEnabled: false,
        textureResolution: 512,
        particleBudget: 100,
      };
    case "mid":
      return {
        renderScale: 0.85,
        frameBudgetMs: 24,
        shadowsEnabled: true,
        textureResolution: 1024,
        particleBudget: 400,
      };
    case "high":
      return {
        renderScale: 1,
        frameBudgetMs: 16,
        shadowsEnabled: true,
        textureResolution: 2048,
        particleBudget: 1000,
      };
  }
}

/** The lowest render scale the governor ever asks for. */
export const MIN_RENDER_SCALE = 0.5;

/** The frame budget of a degraded device: 30 fps. */
export const DEGRADED_FRAME_BUDGET_MS = 33;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/**
 * The quality profile for the current sensors.
 *
 * The tier sets the baseline; thermal, memory and battery pressure lower
 * it. The order matters: the harshest signal wins.
 */
export function governQuality(input: GovernorInput): QualityProfile {
  const base = baseProfile(input.tier);

  const thermalDegraded = input.thermal === "serious" || input.thermal === "critical";
  const memoryDegraded = input.memoryPressure !== "normal";
  const degraded = thermalDegraded || memoryDegraded || input.batterySaver === true;

  if (!degraded) return base;

  return {
    renderScale: clamp(base.renderScale - 0.15, MIN_RENDER_SCALE, 1),
    frameBudgetMs: DEGRADED_FRAME_BUDGET_MS,
    shadowsEnabled: false,
    textureResolution: Math.max(256, Math.round(base.textureResolution / 2)),
    particleBudget: Math.round(base.particleBudget / 2),
  };
}

/**
 * Whether the governor may step the profile back UP.
 *
 * A profile flaps when it steps up and down on every other tick, so the
 * runtime loop only steps up after a sustained healthy period.
 */
export function mayStepUp(healthyForMs: number, minimumHealthyMs = 30_000): boolean {
  return healthyForMs >= minimumHealthyMs;
}
