// <app> — the typed client of the extension message protocol.
//
// The SDK talks to the extension through an injected transport: the caller
// decides how a message travels (a page bridge, externally_connectable, a
// test double). The client has no React and no chrome API, so the same code
// runs in a page, in a test and in the worker.
//
// The envelope mirrors src/contracts/protocol.ts. The SDK cannot import it —
// packages/ is shared with web and mobile — so the shapes are declared here
// and the extension stays the authority: it validates every request it
// receives, so a drift between the two ends fails loudly instead of silently.

/** The version of the protocol the client speaks. */
export const PROTOCOL_VERSION = 1;

/** One message. */
export interface SdkEnvelope {
  readonly v: number;
  readonly id: string;
  readonly correlationId: string;
  readonly type: string;
  readonly payload: unknown;
}

/* eslint-disable no-unused-vars -- the core ESLint rule counts the parameters
   of a type signature as unused values. They are types, and the rule that
   understands them does not reach this package. */
/** How a message travels. The caller injects it. */
export interface SdkTransport {
  send(message: SdkEnvelope): Promise<unknown>;
}

/** A failure, as a value. The code comes from the error code contract. */
export interface SdkError {
  readonly code: string;
  readonly message: string;
}

/** The answer of a call: a value or an error, never an exception. */
export type SdkResult<T> =
  { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: SdkError };

/** A note, as it travels. */
export interface SdkNote {
  readonly id: string;
  readonly title: string;
  readonly body: string;
  readonly createdAt: string;
  readonly updatedAt: string;
}

/** The client of the extension. */
export interface SdkClient {
  /** Every note, newest last. */
  listNotes(): Promise<SdkResult<SdkNote[]>>;
  /** Stores a note and answers with the stored one. */
  addNote(note: { title: string; body: string }): Promise<SdkResult<SdkNote>>;
}
/* eslint-enable no-unused-vars */

/** The answer of the extension, before it is read. */
interface RawResponse {
  readonly v: number;
  readonly id: string;
  readonly ok: boolean;
  readonly payload: unknown;
}

let counter = 0;

/** An id for a message. Unique enough to match an answer with its request. */
function nextId(): string {
  const random = globalThis.crypto?.randomUUID?.();
  if (random) return random;
  counter += 1;
  return `${Date.now().toString(36)}-${counter.toString(36)}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** The answer of the extension, when it is a message. */
function readResponse(value: unknown): RawResponse | undefined {
  if (!isRecord(value)) return undefined;
  const { v, id, ok } = value;
  if (typeof v !== "number" || typeof id !== "string" || typeof ok !== "boolean") return undefined;
  if (!("payload" in value)) return undefined;
  return { v, id, ok, payload: value.payload };
}

/** The error of an answer, when it carries one. */
function readError(payload: unknown): SdkError {
  if (isRecord(payload)) {
    const code = payload["code"];
    const message = payload["message"];
    if (typeof code === "string" && typeof message === "string") return { code, message };
  }
  return { code: "internal", message: "The extension answered with an unreadable error." };
}

/** A note, when it is one. */
function readNote(value: unknown): SdkNote | undefined {
  if (!isRecord(value)) return undefined;
  const { id, title, body, createdAt, updatedAt } = value;
  if (typeof id !== "string" || typeof title !== "string" || typeof body !== "string")
    return undefined;
  if (typeof createdAt !== "string" || typeof updatedAt !== "string") return undefined;
  return { id, title, body, createdAt, updatedAt };
}

/**
 * The client of the extension.
 *
 * Every call returns a result: the extension answers with a typed error, and
 * a transport that fails is a failure of the call, not an exception for the
 * caller to catch.
 */
export function createSdkClient(transport: SdkTransport): SdkClient {
  async function exchange(type: string, payload: unknown): Promise<SdkResult<unknown>> {
    const id = nextId();
    const envelope: SdkEnvelope = {
      v: PROTOCOL_VERSION,
      id,
      correlationId: nextId(),
      type,
      payload,
    };

    let answer: unknown;
    try {
      answer = await transport.send(envelope);
    } catch {
      return { ok: false, error: { code: "internal", message: "The extension did not answer." } };
    }

    const response = readResponse(answer);
    if (!response) {
      return { ok: false, error: { code: "internal", message: "The answer is not a message." } };
    }
    if (response.id !== id) {
      return {
        ok: false,
        error: { code: "internal", message: "The answer does not match the request." },
      };
    }
    if (!response.ok) {
      return { ok: false, error: readError(response.payload) };
    }
    return { ok: true, value: response.payload };
  }

  return {
    listNotes: async () => {
      const result = await exchange("notes.list", {});
      if (!result.ok) return result;

      const payload = result.value;
      if (!isRecord(payload) || !Array.isArray(payload["notes"])) {
        return {
          ok: false,
          error: { code: "internal", message: "The answer is not a note list." },
        };
      }
      const notes: SdkNote[] = [];
      for (const raw of payload["notes"]) {
        const note = readNote(raw);
        if (!note) {
          return {
            ok: false,
            error: { code: "internal", message: "The answer holds an unreadable note." },
          };
        }
        notes.push(note);
      }
      return { ok: true, value: notes };
    },

    addNote: async (note) => {
      const result = await exchange("notes.add", { title: note.title, body: note.body });
      if (!result.ok) return result;

      const payload = result.value;
      const stored = isRecord(payload) ? readNote(payload["note"]) : undefined;
      if (!stored) {
        return { ok: false, error: { code: "internal", message: "The answer holds no note." } };
      }
      return { ok: true, value: stored };
    },
  };
}
