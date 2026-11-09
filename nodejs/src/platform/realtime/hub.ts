// Realtime is the in-process event hub for WS/SSE delivery. Reference
// parity: Redis pub/sub is the documented evolution path (adapter ready).

export interface RealtimeEvent {
  room: string; // "user:<userID>" (see roomUser) or a topic room
  type: string;
  payload: unknown;
  at: Date;
}

// Subscriber is one room subscription: an async iterator over the events
// pushed to it. cancel() is idempotent and unregisters the subscription.
export interface Subscriber extends AsyncIterableIterator<RealtimeEvent> {
  cancel(): void;
}

// EventBus publishes room events and hands out per-subscriber subscriptions.
// publish never blocks: events are dropped, not queued, when the buffer is
// full — a slow consumer must not stall a use-case.
export interface EventBus {
  publish(ev: RealtimeEvent): void;
  subscribe(room: string): Subscriber;
}

// roomUser is the identity room vocabulary shared by WS and SSE: one room per
// user; events published there reach all that user's live connections.
export const roomUser = (userId: string): string => `user:${userId}`;

class HubSubscriber implements Subscriber {
  #queue: RealtimeEvent[] = [];
  #pending: ((r: IteratorResult<RealtimeEvent>) => void) | null = null;
  #closed = false;

  constructor(
    private readonly buffer: number,
    private readonly onClose: () => void,
  ) {}

  push(ev: RealtimeEvent): void {
    if (this.#closed) return;
    if (this.#pending !== null) {
      const settle = this.#pending;
      this.#pending = null;
      settle({ value: ev, done: false });
      return;
    }
    if (this.#queue.length >= this.buffer) return; // drop on full buffer
    this.#queue.push(ev);
  }

  cancel(): void {
    if (this.#closed) return;
    this.#closed = true;
    const settle = this.#pending;
    this.#pending = null;
    settle?.({ value: undefined, done: true });
    this.onClose();
  }

  next(): Promise<IteratorResult<RealtimeEvent>> {
    if (this.#closed) return Promise.resolve({ value: undefined, done: true });
    if (this.#queue.length > 0) {
      const ev = this.#queue.shift();
      return Promise.resolve({ value: ev as RealtimeEvent, done: false });
    }
    return new Promise((resolve) => {
      this.#pending = resolve;
    });
  }

  return(): Promise<IteratorResult<RealtimeEvent>> {
    this.cancel();
    return Promise.resolve({ value: undefined, done: true });
  }

  throw(err?: unknown): Promise<IteratorResult<RealtimeEvent>> {
    this.cancel();
    return Promise.reject(err);
  }

  [Symbol.asyncIterator](): Subscriber {
    return this;
  }
}

class Hub implements EventBus {
  #subs = new Map<string, Set<HubSubscriber>>();

  constructor(private readonly buffer: number) {}

  publish(ev: RealtimeEvent): void {
    for (const s of this.#subs.get(ev.room) ?? []) s.push(ev);
  }

  subscribe(room: string): Subscriber {
    let set = this.#subs.get(room);
    if (set === undefined) {
      set = new Set();
      this.#subs.set(room, set);
    }
    const sub = new HubSubscriber(this.buffer, () => {
      const cur = this.#subs.get(room);
      if (cur === undefined) return;
      cur.delete(sub);
      if (cur.size === 0) this.#subs.delete(room);
    });
    set.add(sub);
    return sub;
  }
}

// newHub builds an EventBus with per-subscriber buffer (<=0 => 16).
export const newHub = (buffer = 16): EventBus => new Hub(buffer <= 0 ? 16 : buffer);
