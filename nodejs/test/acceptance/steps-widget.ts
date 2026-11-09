// Widget SSE steps (steps_widgets.go parity).

import { When, Then } from "@cucumber/cucumber";
import { pair } from "./support/http.js";
import { SseStream } from "./support/world.js";
import type { HarnessWorld } from "./support/world.js";

When(/^I open the event stream for "([^"]*)"$/, async function (this: HarnessWorld, who: string) {
  const p = pair(this, who);
  const ctl = new AbortController();
  const res = await fetch(`${this.app.base}/api/v1/widgets/events`, {
    headers: { Authorization: `Bearer ${p.access}`, Accept: "text/event-stream" },
    signal: ctl.signal,
  });
  // The handler subscribes to the hub before its first byte, so this
  // handshake read guarantees the subscription is live before any subsequent
  // create/update/delete can publish.
  const reader = res.body!.getReader();
  const dec = new TextDecoder();
  let buf = "";
  while (!buf.includes("retry: 2000\n\n")) {
    const { value, done } = await reader.read();
    if (done) {
      ctl.abort();
      throw new Error(`waiting for the "${who}" SSE handshake`);
    }
    buf += dec.decode(value, { stream: true });
  }
  const leftover = buf.slice(buf.indexOf("retry: 2000\n\n") + "retry: 2000\n\n".length);
  this.sse.set(who, new SseStream(ctl, reader, leftover));
});

Then(
  /^the event stream of "([^"]*)" shows a "([^"]*)" event$/,
  async function (this: HarnessWorld, who: string, want: string) {
    const stream = this.sse.get(who);
    if (stream === undefined) {
      throw new Error(
        `no open event stream for "${who}": open it with 'I open the event stream for "${who}"' first`,
      );
    }
    const frame = await stream.next();
    const first = frame.split("\n", 1)[0] ?? "";
    if (!first.startsWith(`event: ${want}`)) {
      throw new Error(`want a "${want}" event, got ${JSON.stringify(frame)}`);
    }
  },
);
