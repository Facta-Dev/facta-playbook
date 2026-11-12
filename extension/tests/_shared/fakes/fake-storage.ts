// <app> — the fake storage area.
//
// An in-memory StorageArea: the typed store and the policy reader are tested
// against it, so no chrome.storage is ever touched in a test.

import { type StorageArea } from "../../../src/platform/storage";

export interface FakeStorage extends StorageArea {
  /** The stored values, as they were written. */
  readonly values: () => Record<string, unknown>;
}

export function createFakeStorage(initial: Record<string, unknown> = {}): FakeStorage {
  const values = new Map<string, unknown>(Object.entries(initial));

  return {
    get<T>(key: string): Promise<T | undefined> {
      return Promise.resolve(values.get(key) as T | undefined);
    },
    set<T>(key: string, value: T): Promise<void> {
      values.set(key, value);
      return Promise.resolve();
    },
    remove(key: string): Promise<void> {
      values.delete(key);
      return Promise.resolve();
    },
    values: () => Object.fromEntries(values),
  };
}
