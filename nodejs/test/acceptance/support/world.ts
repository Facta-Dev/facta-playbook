// Features-support: the per-scenario world carries the harness state; the
// generic HTTP steps own the request/assertion helpers. (steps_http.go parity)

import { setWorldConstructor, World } from "@cucumber/cucumber";
import type { AppHandle } from "../app.js";

// HttpResult is the most recent HTTP exchange.
export interface HttpResult {
  status: number;
  body: string;
  location: string;
}

// TokenPair is one remembered login (access + refresh).
export interface TokenPair {
  access: string;
  refresh: string;
}

// StreamState is one open server-sent-event stream: complete frames
// ("event: ...\ndata: ...") queue up as they arrive.
export class SseStream {
  #ctl: AbortController;
  #reader: ReadableStreamDefaultReader<Uint8Array>;
  #buf = "";
  #frames: string[] = [];
  #eof = false;

  constructor(ctl: AbortController, reader: ReadableStreamDefaultReader<Uint8Array>, buf: string) {
    this.#ctl = ctl;
    this.#reader = reader;
    this.#buf = buf;
    void this.#pump();
  }

  // pump drains the wire: chunk-split on the blank-line frame boundary.
  // close()'s abort rejects the pending read; the pump swallows that.
  #pump = async (): Promise<void> => {
    try {
      while (true) {
        const { value, done } = await this.#reader.read();
        if (done) {
          this.#eof = true;
          return;
        }
        this.#buf += new TextDecoder().decode(value);
        let cut = 0;
        while (true) {
          const idx = this.#buf.indexOf("\n\n", cut);
          if (idx === -1) {
            break;
          }
          this.#frames.push(...this.#frameLines(this.#buf.slice(cut, idx)));
          cut = idx + 2;
        }
        this.#buf = this.#buf.slice(cut);
      }
    } catch (e) {
      if ((e as Error).name !== "AbortError") {
        throw e;
      }
      this.#eof = true;
    }
  };

  // frameLines joins the non-blank lines of one frame with \n (go scanner
  // parity: join frame lines, drop blanks).
  #frameLines(frame: string): string[] {
    return frame.split("\n").filter((l) => l !== "");
  }

  // next returns the next queued frame; throws when none arrives within 5s.
  async next(timeoutMs = 5000): Promise<string> {
    const deadline = Date.now() + timeoutMs;
    while (this.#frames.length === 0 && !this.#eof && Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 25));
    }
    const f = this.#frames.shift();
    if (f === undefined) {
      throw new Error(`no SSE event arrived within ${timeoutMs}ms`);
    }
    return f;
  }

  close(): void {
    this.#ctl.abort();
    void this.#reader.cancel().catch(() => undefined);
  }
}

// HarnessWorld is this scenario's harness state (testState parity).
export class HarnessWorld extends World {
  app!: AppHandle;
  last: HttpResult = { status: 0, body: "", location: "" };
  pairs = new Map<string, TokenPair>();
  vars = new Map<string, string>();
  sse = new Map<string, SseStream>();
}
setWorldConstructor(HarnessWorld);
