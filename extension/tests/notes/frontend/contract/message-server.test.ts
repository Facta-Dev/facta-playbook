// exampletool — the message server.
//
// PLAYBOOK §6.2: the sender is validated FIRST, then the message is parsed
// against the contract, then the handler runs. The server is the boundary
// between the extension contexts and the service worker, so it is tested
// against a fake platform: no test ever touches chrome.*.

import { describe, expect, it } from "vitest";
import { registerMessageServer } from "../../../../src/messaging/server";
import { createNotesHandlers } from "../../../../src/messaging/notes";
import { ResponseSchema } from "../../../../src/contracts/protocol";
import { type Note } from "../../../../src/contracts/notes";
import { createLogger } from "../../../../src/telemetry/logger";
import { asNoteId } from "../../../../src/shared-kernel/ids";
import { createFakeRuntime, createSender } from "../../../_shared/fakes/fake-runtime";

const EXTENSION_ID = "test-extension-id";
const EXTENSION_ORIGIN = `chrome-extension://${EXTENSION_ID}`;

/** The notes the fake port holds. */
function createPort(): {
  list(): Promise<Note[]>;
  add(input: { title: string; body: string }): Promise<Note>;
} {
  const notes: Note[] = [];
  return {
    list(): Promise<Note[]> {
      return Promise.resolve([...notes]);
    },
    add(input: { title: string; body: string }): Promise<Note> {
      const note: Note = {
        id: asNoteId(`note-${notes.length + 1}`),
        title: input.title,
        body: input.body,
        createdAt: "2024-01-01T00:00:00.000Z",
        updatedAt: "2024-01-01T00:00:00.000Z",
      };
      notes.push(note);
      return Promise.resolve(note);
    },
  };
}

/** A registered server and the fake runtime it listens on. */
function createServer() {
  const runtime = createFakeRuntime(EXTENSION_ID);
  const port = createPort();
  registerMessageServer({
    runtime,
    logger: createLogger(),
    senderPolicy: {
      extensionId: EXTENSION_ID,
      extensionOrigin: EXTENSION_ORIGIN,
      contentScriptPatterns: ["https://example.com/*"],
    },
    handlers: createNotesHandlers(port),
  });
  return { runtime, port };
}

/** A sender of an extension page: the side panel or the options page. */
const uiSender = createSender({ id: EXTENSION_ID, url: `${EXTENSION_ORIGIN}/sidepanel.html` });

describe("message server", () => {
  it("registers one listener on the platform runtime", () => {
    const { runtime } = createServer();
    expect(runtime.registered()).toBe(true);
  });

  it("refuses a message from another extension", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(
      await runtime.respond(
        { v: 1, id: "request-1", correlationId: "correlation-1", type: "notes.list", payload: {} },
        createSender({ id: "another-extension", url: `${EXTENSION_ORIGIN}/sidepanel.html` }),
      ),
    );

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("forbidden-sender");
  });

  it("refuses a message from a foreign origin", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(
      await runtime.respond(
        { v: 1, id: "request-1", correlationId: "correlation-1", type: "notes.list", payload: {} },
        createSender({ id: EXTENSION_ID, url: "https://attacker.example/page" }),
      ),
    );

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("forbidden-sender");
  });

  it("refuses a malformed payload with a typed error", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(
      await runtime.respond(
        {
          v: 1,
          id: "request-1",
          correlationId: "correlation-1",
          type: "notes.add",
          payload: { title: 42, body: "a body" },
        },
        uiSender,
      ),
    );

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("validation-failed");
  });

  it("refuses a message that is not a valid envelope", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(await runtime.respond({ type: "notes.list" }, uiSender));

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("bad-envelope");
  });

  it("refuses a message type that does not exist", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(
      await runtime.respond(
        {
          v: 1,
          id: "request-1",
          correlationId: "correlation-1",
          type: "notes.destroy",
          payload: {},
        },
        uiSender,
      ),
    );

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("unknown-type");
  });

  it("refuses a message type the sender class may not send", async () => {
    const { runtime } = createServer();

    const response = ResponseSchema.parse(
      await runtime.respond(
        {
          v: 1,
          id: "request-1",
          correlationId: "correlation-1",
          type: "notes.add",
          payload: { title: "a title", body: "a body" },
        },
        createSender({ id: EXTENSION_ID, url: "https://example.com/page", tabId: 7, frameId: 0 }),
      ),
    );

    expect(response.ok).toBe(false);
    if (response.ok) throw new Error("expected an error response");
    expect(response.payload.code).toBe("forbidden-sender");
  });

  it("answers a valid request", async () => {
    const { runtime, port } = createServer();
    await port.add({ title: "a title", body: "a body" });

    const response = ResponseSchema.parse(
      await runtime.respond(
        { v: 1, id: "request-1", correlationId: "correlation-1", type: "notes.list", payload: {} },
        uiSender,
      ),
    );

    expect(response.ok).toBe(true);
    if (!response.ok) throw new Error("expected a successful response");
    expect(response.type).toBe("notes.list");
    if (response.type !== "notes.list") throw new Error("expected a notes.list response");
    expect(response.payload.notes).toHaveLength(1);
  });
});
