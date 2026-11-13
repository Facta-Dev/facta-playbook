// <app> — the mobile ports.
//
// A port is a plain TypeScript interface: the shape the application layer
// expects from the outside world. The implementation lives in an adapter
// (packages/feature-*/src/adapters/out) and is wired in the composition
// root (app/App.tsx).
//
// There is no implementation in this package on purpose: the core stays
// pure TypeScript, so it can be type-checked and unit-tested without a
// device, an emulator or a native module.

/** Cancels a subscription. */
export type Unsub = () => void;

/** The state of a runtime permission. */
export const PERMISSION_STATES = ["undetermined", "denied", "granted"] as const;

/** The state of a runtime permission. */
export type PermissionState = (typeof PERMISSION_STATES)[number];

/** The device registration sent to the push provider. */
export interface DeviceRegistration {
  /** The provider token. */
  readonly token: string;
  /** The platform the token belongs to. */
  readonly platform: "android" | "ios";
}

/** A notification that reached the app. */
export interface IncomingNotification {
  /** The provider identifier of the notification. */
  readonly id: string;
  /** The title shown in the tray. */
  readonly title: string;
  /** The body shown in the tray. */
  readonly body: string;
  /** The untrusted payload — validate every field before you use it. */
  readonly data: Readonly<Record<string, string>>;
}

/** The connectivity of the device. */
export const NETWORK_STATES = ["offline", "cellular", "wifi", "unknown"] as const;

/** The connectivity of the device. */
export type NetworkState = (typeof NETWORK_STATES)[number];

/** A deep link, parsed. The payload is untrusted. */
export interface ParsedLink {
  /** The path of the link, without the query string. */
  readonly path: string;
  /** The query parameters of the link. */
  readonly params: Readonly<Record<string, string>>;
}

/** The push notification capability of the device. */
export interface PushPort {
  /** The current permission state, without prompting. */
  permissionState(): Promise<PermissionState>;
  /** Asks the user for the permission. */
  requestPermission(): Promise<PermissionState>;
  /** Registers the device with the push provider. */
  registerDevice(): Promise<DeviceRegistration>;
  /** Removes the device registration. */
  unregisterDevice(): Promise<void>;
  /** Subscribes to notifications received in the foreground. */
  onNotification(callback: (notification: IncomingNotification) => void): Unsub;
  /** Subscribes to notification taps. */
  onTap(callback: (notification: IncomingNotification) => void): Unsub;
}

/** The secure key/value store. Secrets only — never plain preferences. */
export interface SecureStorePort {
  /** Reads a value, or null when the key is unknown. */
  get(key: string): Promise<string | null>;
  /** Writes a value. */
  set(key: string, value: string): Promise<void>;
  /** Removes a value. */
  remove(key: string): Promise<void>;
}

/** The connectivity of the device. */
export interface NetworkStatePort {
  /** The current state. */
  current(): NetworkState;
  /** Subscribes to state changes. */
  onChange(callback: (state: NetworkState) => void): Unsub;
}

/** The deep links the app answers to. */
export interface DeepLinkPort {
  /** The link the app was cold-started with, if any. */
  initial(): Promise<ParsedLink | null>;
  /** Subscribes to links that arrive while the app runs. */
  onLink(callback: (link: ParsedLink) => void): Unsub;
}
