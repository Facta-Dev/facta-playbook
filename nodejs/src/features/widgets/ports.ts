// Features widgets ports: what the use-case needs from the outside world,
// implemented by adapters (postgres.ts) and wired in bin/api.ts.

import type { RealtimeEvent, Subscriber } from "#platform/realtime/index.js";
import type { Widget, WidgetKind } from "./domain.js";

// Store persists owner-scoped widgets (impl: postgres.ts).
export interface Store {
  insert(w: Widget): Promise<void>;
  list(ownerId: string, limit: number, offset: number): Promise<Widget[]>;
  get(id: string, ownerId: string): Promise<Widget | null>;
  update(id: string, ownerId: string, name: string, kind: WidgetKind): Promise<void>;
  delete(id: string, ownerId: string): Promise<void>;
}

// Publisher is the realtime hub port: publish lifecycle events and hand out
// per-subscriber subscriptions for the SSE stream (impl: realtime hub).
export interface Publisher {
  publish(ev: RealtimeEvent): void;
  subscribe(room: string): Subscriber;
}
