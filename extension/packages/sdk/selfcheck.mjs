// <app> — the self-check of the SDK.
//
// The SDK has no dependencies and no imports, so node runs it as it is:
//
//   node packages/sdk/selfcheck.mjs
//
// It checks the paths that can break: a good answer, an error answer, an
// answer that belongs to another request, and a transport that fails.
import assert from "node:assert/strict";
import { createSdkClient } from "./src/index.ts";

const NOTE = {
  id: "note-1",
  title: "A title",
  body: "A body",
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
};

/** A transport that answers with a fixed message. */
function transportOf(answer) {
  return {
    send: async (message) => ({
      ...answer,
      id: message.id,
      correlationId: message.correlationId,
    }),
  };
}

const listed = await createSdkClient(
  transportOf({ v: 1, type: "notes.list", ok: true, payload: { notes: [NOTE] } }),
).listNotes();
assert.equal(listed.ok, true);
assert.deepEqual(listed.value, [NOTE]);

const added = await createSdkClient(
  transportOf({ v: 1, type: "notes.add", ok: true, payload: { note: NOTE } }),
).addNote({ title: NOTE.title, body: NOTE.body });
assert.equal(added.ok, true);
assert.deepEqual(added.value, NOTE);

const failed = await createSdkClient(
  transportOf({
    v: 1,
    type: "notes.add",
    ok: false,
    payload: { code: "validation-failed", message: "The title is too long." },
  }),
).addNote({ title: NOTE.title, body: NOTE.body });
assert.equal(failed.ok, false);
assert.equal(failed.error.code, "validation-failed");

const mismatched = await createSdkClient({
  send: async () => ({
    v: 1,
    id: "another-request",
    correlationId: "another-correlation",
    type: "notes.list",
    ok: true,
    payload: { notes: [] },
  }),
}).listNotes();
assert.equal(mismatched.ok, false);

const broken = await createSdkClient({
  send: async () => {
    throw new Error("the bridge is down");
  },
}).listNotes();
assert.equal(broken.ok, false);
