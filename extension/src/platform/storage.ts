// <app> — the storage transport.
//
// storage.local is durable, storage.session is volatile and storage.managed
// is read-only administrator policy (PLAYBOOK §2.3). Nothing outside this
// folder talks to the extension API directly.

/** One storage area, addressed by key. */
export interface StorageArea {
  get<T>(key: string): Promise<T | undefined>;
  set<T>(key: string, value: T): Promise<void>;
  remove(key: string): Promise<void>;
}

/** The administrator policy area: a flat object of policy keys. */
export interface ManagedArea {
  /** The whole managed object, the properties being the policy keys. */
  read(): Promise<Record<string, unknown>>;
}

export interface StorageAdapter {
  readonly local: StorageArea;
  readonly session: StorageArea;
  readonly managed: ManagedArea;
}

export function createStorage(): StorageAdapter {
  return {
    local: {
      async get<T>(key: string): Promise<T | undefined> {
        const values = await chrome.storage.local.get(key);
        return values[key] as T | undefined;
      },
      async set<T>(key: string, value: T): Promise<void> {
        await chrome.storage.local.set({ [key]: value });
      },
      async remove(key: string): Promise<void> {
        await chrome.storage.local.remove(key);
      },
    },
    session: {
      async get<T>(key: string): Promise<T | undefined> {
        const values = await chrome.storage.session.get(key);
        return values[key] as T | undefined;
      },
      async set<T>(key: string, value: T): Promise<void> {
        await chrome.storage.session.set({ [key]: value });
      },
      async remove(key: string): Promise<void> {
        await chrome.storage.session.remove(key);
      },
    },
    managed: {
      async read(): Promise<Record<string, unknown>> {
        return chrome.storage.managed.get(null);
      },
    },
  };
}

/**
 * A durable area whose writes are mirrored into a volatile one.
 *
 * The mirror is debounced: the durable write is the one that matters, the
 * session copy is a cache the worker may rebuild at any time.
 *
 * ponytail: a debounce timer does not survive the worker, so a mirror write
 * can be lost when the worker is terminated. Drop the mirror (or flush it
 * from an alarm) when that cache ever becomes load-bearing.
 */
export function createWriteThrough(
  durable: StorageArea,
  mirror: StorageArea,
  delayMs = 250,
): StorageArea {
  const pending = new Map<string, unknown>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  const flush = (): void => {
    timer = undefined;
    const writes = [...pending.entries()];
    pending.clear();
    void Promise.all(writes.map(([key, value]) => mirror.set(key, value))).catch(() => undefined);
  };

  return {
    get: (key) => durable.get(key),
    async set<T>(key: string, value: T): Promise<void> {
      await durable.set(key, value);
      pending.set(key, value);
      timer ??= setTimeout(flush, delayMs);
    },
    async remove(key: string): Promise<void> {
      await durable.remove(key);
      pending.delete(key);
      void mirror.remove(key);
    },
  };
}
