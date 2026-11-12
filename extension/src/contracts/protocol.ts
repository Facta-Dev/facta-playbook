// <app> — the message protocol.
//
// One contract, both ends validate with it (PLAYBOOK §6.1). The envelope is
// { v, id, type, correlationId, payload }; unknown versions and unknown
// types are rejected with a typed error, never with an exception.

import { z } from "zod";
import { asCorrelationId, asRequestId } from "../shared-kernel/ids";
import { ErrorCodeSchema } from "./errors";
import { NoteSchema, NOTE_BODY_MAX_LENGTH, NOTE_TITLE_MAX_LENGTH } from "./notes";

/** The version of the protocol. Bumped on a breaking change. */
export const PROTOCOL_VERSION = 1;

/** The classes of context allowed to send a message (PLAYBOOK §6.2). */
export const SENDER_CLASSES = ["ui", "content"] as const;

export type SenderClass = (typeof SENDER_CLASSES)[number];

const envelope = {
  v: z.literal(PROTOCOL_VERSION),
  id: z.string().min(1).transform(asRequestId),
  correlationId: z.string().min(1).transform(asCorrelationId),
};

// --- payloads ---------------------------------------------------------------

export const NotesListPayloadSchema = z.strictObject({});
export type NotesListPayload = z.infer<typeof NotesListPayloadSchema>;

export const NotesListResultSchema = z.strictObject({ notes: z.array(NoteSchema) });
export type NotesListResult = z.infer<typeof NotesListResultSchema>;

export const NotesAddPayloadSchema = z.strictObject({
  title: z.string().min(1).max(NOTE_TITLE_MAX_LENGTH),
  body: z.string().max(NOTE_BODY_MAX_LENGTH),
});
export type NotesAddPayload = z.infer<typeof NotesAddPayloadSchema>;

export const NotesAddResultSchema = z.strictObject({ note: NoteSchema });
export type NotesAddResult = z.infer<typeof NotesAddResultSchema>;

// --- envelopes --------------------------------------------------------------

export const NotesListRequestSchema = z.strictObject({
  ...envelope,
  type: z.literal("notes.list"),
  payload: NotesListPayloadSchema,
});

export const NotesAddRequestSchema = z.strictObject({
  ...envelope,
  type: z.literal("notes.add"),
  payload: NotesAddPayloadSchema,
});

/** Every request the extension answers. */
export const RequestSchema = z.discriminatedUnion("type", [
  NotesListRequestSchema,
  NotesAddRequestSchema,
]);
export type Request = z.infer<typeof RequestSchema>;

export const NotesListResponseSchema = z.strictObject({
  ...envelope,
  type: z.literal("notes.list"),
  ok: z.literal(true),
  payload: NotesListResultSchema,
});

export const NotesAddResponseSchema = z.strictObject({
  ...envelope,
  type: z.literal("notes.add"),
  ok: z.literal(true),
  payload: NotesAddResultSchema,
});

export const SuccessResponseSchema = z.discriminatedUnion("type", [
  NotesListResponseSchema,
  NotesAddResponseSchema,
]);

export const ErrorResponseSchema = z.strictObject({
  ...envelope,
  type: z.string().min(1),
  ok: z.literal(false),
  payload: z.strictObject({ code: ErrorCodeSchema, message: z.string() }),
});

/** Every answer the extension sends, successful or not. */
export const ResponseSchema = z.union([SuccessResponseSchema, ErrorResponseSchema]);
export type Response = z.infer<typeof ResponseSchema>;

/**
 * The loose view of a message, read before the message is validated. The
 * server needs the type to pick the schema; the message itself is then
 * parsed against the contract, which rejects the fields this view ignores.
 */
export const MessageHeaderSchema = z.object({
  v: z.literal(PROTOCOL_VERSION),
  id: z.string().min(1),
  correlationId: z.string().min(1),
  type: z.string().min(1),
});

export type MessageHeader = z.infer<typeof MessageHeaderSchema>;

// --- the message table ------------------------------------------------------

export const PROTOCOL = {
  "notes.list": {
    type: "notes.list",
    response: NotesListResponseSchema,
    senders: ["ui", "content"],
  },
  "notes.add": {
    type: "notes.add",
    response: NotesAddResponseSchema,
    senders: ["ui"],
  },
} as const;

export type ProtocolType = keyof typeof PROTOCOL;

/** One entry of the message table, as the server reads it. */
export interface ProtocolDefinition {
  readonly type: ProtocolType;
  readonly response: z.ZodType;
  readonly senders: readonly SenderClass[];
}

const DEFINITIONS: Record<string, ProtocolDefinition> = {
  "notes.list": PROTOCOL["notes.list"],
  "notes.add": PROTOCOL["notes.add"],
};

/** The definition of a message type, or undefined when the type is unknown. */
export function definition(type: string): ProtocolDefinition | undefined {
  return DEFINITIONS[type];
}

export type ProtocolPayloads = {
  "notes.list": NotesListPayload;
  "notes.add": NotesAddPayload;
};

export type ProtocolResults = {
  "notes.list": NotesListResult;
  "notes.add": NotesAddResult;
};
