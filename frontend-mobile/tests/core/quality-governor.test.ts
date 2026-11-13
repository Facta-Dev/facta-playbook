// <app> — the quality governor.
//
// The governor is a pure function, so the whole policy is a unit test: the
// tier sets the baseline and the sensors lower it.

import { describe, expect, it } from "vitest";
import {
  DEGRADED_FRAME_BUDGET_MS,
  baseProfile,
  governQuality,
  mayStepUp,
  resolveDeviceTier,
  type GovernorInput,
} from "@<scope>/core";

const healthy: GovernorInput = {
  tier: "mid",
  thermal: "nominal",
  memoryPressure: "normal",
};

describe("resolveDeviceTier", () => {
  it("trusts the low-RAM flag", () => {
    expect(resolveDeviceTier({ lowRamDevice: true, totalRamGb: 12 })).toBe("low");
  });

  it("reads the RAM size", () => {
    expect(resolveDeviceTier({ totalRamGb: 2 })).toBe("low");
    expect(resolveDeviceTier({ totalRamGb: 5 })).toBe("mid");
    expect(resolveDeviceTier({ totalRamGb: 12 })).toBe("high");
  });

  it("falls back to mid when there is no signal", () => {
    expect(resolveDeviceTier({})).toBe("mid");
  });

  it("lets the measurement lower the hint, never raise it", () => {
    expect(resolveDeviceTier({ totalRamGb: 12, measuredFrameP95Ms: 40 })).toBe("low");
    expect(resolveDeviceTier({ totalRamGb: 12, measuredFrameP95Ms: 24 })).toBe("mid");
    expect(resolveDeviceTier({ totalRamGb: 12, measuredFrameP95Ms: 10 })).toBe("high");
  });
});

describe("the quality profiles", () => {
  it("gives every tier a profile inside the budgets", () => {
    for (const tier of ["low", "mid", "high"] as const) {
      const profile = baseProfile(tier);
      expect(profile.renderScale).toBeGreaterThanOrEqual(0.5);
      expect(profile.renderScale).toBeLessThanOrEqual(1);
      expect(profile.frameBudgetMs).toBeGreaterThanOrEqual(16);
      expect(profile.frameBudgetMs).toBeLessThanOrEqual(33);
    }
  });

  it("keeps the shadows off on the low tier", () => {
    expect(baseProfile("low").shadowsEnabled).toBe(false);
  });
});

describe("governQuality", () => {
  it("returns the tier baseline when the device is healthy", () => {
    expect(governQuality(healthy)).toEqual(baseProfile("mid"));
  });

  it("steps down when the device is hot", () => {
    const profile = governQuality({ ...healthy, thermal: "serious" });
    expect(profile.frameBudgetMs).toBe(DEGRADED_FRAME_BUDGET_MS);
    expect(profile.shadowsEnabled).toBe(false);
    expect(profile.renderScale).toBeLessThan(baseProfile("mid").renderScale);
  });

  it("steps down under memory pressure", () => {
    const profile = governQuality({ ...healthy, memoryPressure: "warning" });
    expect(profile.shadowsEnabled).toBe(false);
    expect(profile.particleBudget).toBeLessThan(baseProfile("mid").particleBudget);
  });

  it("steps down in the battery saver", () => {
    const profile = governQuality({ ...healthy, batterySaver: true });
    expect(profile.frameBudgetMs).toBe(DEGRADED_FRAME_BUDGET_MS);
  });

  it("never asks for a render scale below the floor", () => {
    const profile = governQuality({ tier: "low", thermal: "critical", memoryPressure: "critical" });
    expect(profile.renderScale).toBeGreaterThanOrEqual(0.5);
  });
});

describe("mayStepUp", () => {
  it("waits for a sustained healthy period", () => {
    expect(mayStepUp(1_000)).toBe(false);
    expect(mayStepUp(30_000)).toBe(true);
  });
});
