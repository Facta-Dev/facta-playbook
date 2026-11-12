// <app> — the notes repository over the message protocol.
//
// The side panel reads and writes through the service worker, never through
// chrome.storage directly: the worker is the only writer of the record
// (PLAYBOOK §7.1), so two contexts cannot overwrite each other.

import { type RpcClient } from "../../../../messaging/client";
import { err, ok, type Result } from "../../../../shared-kernel/result";
import { type NoteRepository, type NotesError } from "../../application/ports";
import { type Note, type NoteDraft } from "../../domain/note";

/**
 * A transport failure is not a business rule: every error the client can
 * report (forbidden sender, invalid payload, unreachable worker) reaches the
 * application as one storage-class failure with the detail preserved.
 */
function transportError(message: string): NotesError {
  return { code: "storage-failed", message };
}

export function createRpcNoteRepository(client: RpcClient): NoteRepository {
  return {
    list: async (): Promise<Result<Note[], NotesError>> => {
      const answer = await client.request("notes.list", {});
      if (!answer.ok) return err(transportError(answer.error.message));
      return ok(answer.value.notes);
    },
    add: async (draft: NoteDraft): Promise<Result<Note, NotesError>> => {
      const answer = await client.request("notes.add", { title: draft.title, body: draft.body });
      if (!answer.ok) return err(transportError(answer.error.message));
      return ok(answer.value.note);
    },
  };
}
