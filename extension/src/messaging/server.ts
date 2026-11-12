// <app> — the typed message server.
//
// PLAYBOOK §6.2: the sender is validated FIRST, then the message is parsed
// against the contract, then the handler runs. A malformed message is
// answered with a typed error; it never throws inside the worker.

import {
  ErrorResponseSchema,
  MessageHeaderSchema,
  PROTOCOL_VERSION,
  RequestSchema,
  definition as protocolDefinition,
  type ProtocolPayloads,
  type ProtocolResults,
  type ProtocolType,
  type SenderClass,
} from "../contracts/protocol";
import { type PlatformSender, type RuntimePort } from "../platform/runtime";
import { classifySender, type SenderPolicy } from "../security/sender";
import { type ErrorCode } from "../shared-kernel/errors";
import { type Logger } from "../telemetry/logger";

export interface HandlerContext {
  /** The class of the context that sent the message. */
  readonly senderClass: SenderClass;
  /** The correlation id of the request. */
  readonly correlationId: string;
  /** The sender, as the platform reported it. */
  readonly sender: PlatformSender;
}

export type MessageHandler<TPayload, TResult> = (
  payload: TPayload,
  context: HandlerContext,
) => Promise<TResult>;

/** The handler table: one handler per message type. */
export type HandlerTable = {
  [K in ProtocolType]: MessageHandler<ProtocolPayloads[K], ProtocolResults[K]>;
};

export interface MessageServerOptions {
  readonly runtime: RuntimePort;
  readonly handlers: HandlerTable;
  readonly senderPolicy: SenderPolicy;
  readonly logger: Logger;
}

export function registerMessageServer(options: MessageServerOptions): void {
  const { runtime, handlers, senderPolicy, logger } = options;

  /** The handler of a message type, with the payload type erased. */
  const dispatch = async (
    type: ProtocolType,
    payload: unknown,
    context: HandlerContext,
  ): Promise<unknown> => {
    const handler = handlers[type] as unknown as (
      input: unknown,
      ctx: HandlerContext,
    ) => Promise<unknown>;
    return handler(payload, context);
  };

  runtime.onMessage(async (message, sender) => {
    // 1. The sender, before anything is parsed.
    const classified = classifySender(sender, senderPolicy);
    if (!classified.ok) {
      logger.log("warn", "message refused", { code: classified.error, senderId: sender.id });
      return errorResponse(
        classified.error,
        "the sender may not talk to this extension",
        fallbackMeta(message),
      );
    }

    // 2. The envelope, then the payload.
    const header = MessageHeaderSchema.safeParse(message);
    if (!header.success) {
      return errorResponse(
        "bad-envelope",
        "the message is not a valid envelope",
        fallbackMeta(message),
      );
    }
    const meta: EnvelopeMeta = {
      id: header.data.id,
      correlationId: header.data.correlationId,
      type: header.data.type,
    };
    const entry = protocolDefinition(header.data.type);
    if (entry === undefined) {
      return errorResponse("unknown-type", "the message type is not supported", meta);
    }
    if (!entry.senders.includes(classified.value)) {
      logger.log("warn", "message refused", {
        type: entry.type,
        senderClass: classified.value,
      });
      return errorResponse("forbidden-sender", "the sender may not send this message", meta);
    }
    const request = RequestSchema.safeParse(message);
    if (!request.success) {
      logger.log("warn", "message refused", {
        type: entry.type,
        issue: request.error.issues[0]?.code,
      });
      return errorResponse("validation-failed", "the message payload is invalid", meta);
    }

    // 3. The handler.
    const context: HandlerContext = {
      senderClass: classified.value,
      correlationId: header.data.correlationId,
      sender,
    };
    let result: unknown;
    try {
      result = await dispatch(entry.type, request.data.payload, context);
    } catch (error) {
      logger.log("error", "the handler failed", { type: entry.type, error: String(error) });
      return errorResponse("internal", "the handler failed", meta);
    }
    const response = entry.response.safeParse({
      v: PROTOCOL_VERSION,
      id: header.data.id,
      correlationId: header.data.correlationId,
      type: entry.type,
      ok: true,
      payload: result,
    });
    if (!response.success) {
      logger.log("error", "the handler returned an invalid payload", { type: entry.type });
      return errorResponse("internal", "the handler returned an invalid payload", meta);
    }
    return response.data;
  });
}

interface EnvelopeMeta {
  readonly id: string;
  readonly correlationId: string;
  readonly type: string;
}

function errorResponse(code: ErrorCode, message: string, meta: EnvelopeMeta): unknown {
  return ErrorResponseSchema.parse({
    v: PROTOCOL_VERSION,
    id: meta.id,
    correlationId: meta.correlationId,
    type: meta.type,
    ok: false,
    payload: { code, message },
  });
}

/** The identity of a message that could not be read. */
function fallbackMeta(message: unknown): EnvelopeMeta {
  return {
    id: rawString(message, "id") ?? crypto.randomUUID(),
    correlationId: rawString(message, "correlationId") ?? crypto.randomUUID(),
    type: rawString(message, "type") ?? "unknown",
  };
}

function rawString(message: unknown, key: string): string | undefined {
  if (typeof message !== "object" || message === null) return undefined;
  const value = (message as Record<string, unknown>)[key];
  return typeof value === "string" && value.length > 0 ? value : undefined;
}
