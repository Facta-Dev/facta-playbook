// Realtime hub unit tests (backend-go hub_test.go parity).

import { describe, expect, test } from "vitest";
import { newHub, roomUser, type RealtimeEvent } from "./index.js";

const ev = (type: string, room: string): RealtimeEvent => ({
  room,
  type,
  payload: { id: "w-1" },
  at: new Date(0),
});

describe("realtime hub", () => {
  test("subscribe/publish/cancel delivers to the room", async () => {
    const h = newHub(4);
    const s = h.subscribe(roomUser("u-1"));
    h.publish(ev("widget.created", roomUser("u-1")));
    const r = await s.next();
    expect(r.done).toBe(false);
    expect((r.value as RealtimeEvent).type).toBe("widget.created");
    s.cancel();
  });

  test("rooms are isolated — no cross-room leak", async () => {
    const h = newHub(4);
    const s = h.subscribe(roomUser("a"));
    h.publish(ev("nope", roomUser("b")));
    const r = await Promise.race([
      s.next(),
      new Promise<"timeout">((res) => setTimeout(() => res("timeout"), 50)),
    ]);
    expect(r).toBe("timeout");
    s.cancel();
  });

  test("publish drops when the buffer is full — never blocks", async () => {
    const h = newHub(1);
    const s = h.subscribe(roomUser("a"));
    h.publish(ev("one", roomUser("a"))); // fills the buffer
    h.publish(ev("two", roomUser("a"))); // must drop, not block
    h.publish(ev("three", roomUser("a")));
    const first = await s.next();
    expect((first.value as RealtimeEvent).type).toBe("one");
    // 50ms drain window: nothing more should arrive (the two drops).
    const r = await Promise.race([
      s.next(),
      new Promise<"timeout">((res) => setTimeout(() => res("timeout"), 50)),
    ]);
    expect(r).toBe("timeout");
    s.cancel();
  });

  test("cancel closes the subscription — next() reports done", async () => {
    const h = newHub(1);
    const s = h.subscribe(roomUser("a"));
    s.cancel();
    const r = await s.next();
    expect(r.done).toBe(true);
  });

  test("roomUser prefixes the identity room with user:", () => {
    expect(roomUser("u-1").startsWith("user:")).toBe(true);
  });
});
