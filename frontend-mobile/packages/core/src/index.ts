// <app> — the shared kernel.
//
// Pure TypeScript, no React, no React Native, no Expo. Everything here is
// imported by the app, by the contracts and by every feature package.

export { err, isErr, isOk, ok } from "./result";
export type { Err, Ok, Result } from "./result";

export {
  DEVICE_TIERS,
  LOW_TIER_FRAME_P95_MS,
  MID_TIER_FRAME_P95_MS,
  MIN_OS_API_LEVEL,
  parseDeviceTier,
  resolveDeviceTier,
  tierFromSignals,
  weakerTier,
} from "./device-tier";
export type { DeviceSignals, DeviceTier } from "./device-tier";

export {
  DEGRADED_FRAME_BUDGET_MS,
  MEMORY_PRESSURES,
  MIN_RENDER_SCALE,
  THERMAL_STATES,
  baseProfile,
  governQuality,
  mayStepUp,
} from "./quality-governor";
export type {
  GovernorInput,
  MemoryPressure,
  QualityProfile,
  ThermalState,
} from "./quality-governor";

export { NETWORK_STATES, PERMISSION_STATES } from "./ports";
export type {
  DeepLinkPort,
  DeviceRegistration,
  IncomingNotification,
  NetworkState,
  NetworkStatePort,
  ParsedLink,
  PermissionState,
  PushPort,
  SecureStorePort,
  Unsub,
} from "./ports";
