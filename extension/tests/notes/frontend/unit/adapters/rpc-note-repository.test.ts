// <app> — the notes repository over the message protocol.
//
// The panel reads and writes through the worker (PLAYBOOK §7.1): these tests
// pin the mapping from the protocol answer to the port result, against a
// client that never touches chrome.*.

import { describe, expect, it } from "vitest";
import { createRpcNoteRepository } from "../../../../../src/features/notes/adapters/out/rpc-note-repository";
import { type Note } from "../../../../../src/features/notes/domain/note";
import { type ProtocolType } from "../../../../../src/contracts/protocol";
import { type RpcClient, type RpcError } from "../../../../../src/messaging/client";
import { asNoteId } from "../../../../../src/shared-kernel/ids";
import { err, ok, type Result } from "../../../../../src/shared-kernel/result";

const NOTE: Note = {
  id: asNoteId("note-1"),
  title: "a title",
  body: "a body",
  createdAt: "2024-01-01T00:00:00.000Z",
  updatedAt: "2024-01-01T00:00:00.000Z",
};

type Answer = Result<unknown, RpcError>;

/** A client that answers with the value given for the message type. */
function createClient(answers: Readonly<Record<ProtocolType, Answer>>) {
  const calls: Array<{ type: string; payload: unknown }> = [];
  const client = {
    request: (type: ProtocolType, payload: unknown) => {
      calls.push({ type, payload });
      return Promise.resolve(answers[type]);
    },
  } as unknown as RpcClient;
  return { client, calls };
}

describe("the notes repository over the protocol", () => {
  it("lists through notes.list and returns the notes", async () => {
    const { client, calls } = createClient({
      "notes.list": ok({ notes: [NOTE] }),
      "notes.add": ok({ note: NOTE }),
    });
    const repository = createRpcNoteRepository(client);

    const result = await repository.list();

    expect(result).toEqual(ok([NOTE]));
    expect(calls).toEqual([{ type: "notes.list", payload: {} }]);
  });

  it("stores through notes.add and returns the stored note", async () => {
    const { client, calls } = createClient({
      "notes.list": ok({ notes: [] }),
      "notes.add": ok({ note: NOTE }),
    });
    const repository = createRpcNoteRepository(client);

    const result = await repository.add({ title: "a title", body: "a body" });

    expect(result).toEqual(ok(NOTE));
    expect(calls).toEqual([{ type: "notes.add", payload: { title: "a title", body: "a body" } }]);
  });

  it("turns a refused answer into one storage failure with the detail kept", async () => {
    const { client } = createClient({
      "notes.list": err({ code: "forbidden-sender", message: "the sender is not allowed" }),
      "notes.add": err({ code: "forbidden-sender", message: "the sender is not allowed" }),
    });
    const repository = createRpcNoteRepository(client);

    const result = await repository.list();

    expect(result).toEqual(err({ code: "storage-failed", message: "the sender is not allowed" }));
  });
});
