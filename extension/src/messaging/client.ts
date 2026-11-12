// <app> — the typed message client.
//
// The client never touches chrome.*: it goes through the platform runtime.
// A failed call is a value, never an exception (PLAYBOOK §6.1).

import {
  PROTOCOL_VERSION,
  RequestSchema,
  ResponseSchema,
  type ProtocolPayloads,
  type ProtocolResults,
  type ProtocolType,
} from "../contracts/protocol";
import { type RuntimePort } from "../platform/runtime";
import { type ErrorCode } from "../shared-kernel/errors";
import { err, ok, type Result } from "../shared-kernel/result";

export interface RpcError {
  readonly code: ErrorCode;
  readonly message: string;
}

export interface RpcClient {
  request<K extends ProtocolType>(
    type: K,
    payload: ProtocolPayloads[K],
  ): Promise<Result<ProtocolResults[K], RpcError>>;
}

export function createRpcClient(runtime: RuntimePort): RpcClient {
  return {
    request: async <K extends ProtocolType>(type: K, payload: ProtocolPayloads[K]) => {
      const envelope = {
        v: PROTOCOL_VERSION,
        id: crypto.randomUUID(),
        correlationId: crypto.randomUUID(),
        type,
        payload,
      };
      // Both ends validate with the same schemas (PLAYBOOK §6.1).
      if (!RequestSchema.safeParse(envelope).success) {
        return err({ code: "internal", message: "the request does not match the protocol" });
      }
      let answer: unknown;
      try {
        answer = await runtime.sendMessage(envelope);
      } catch {
        return err({ code: "internal", message: "the service worker did not answer" });
      }
      const parsed = ResponseSchema.safeParse(answer);
      if (!parsed.success) {
        return err({ code: "internal", message: "the answer is not a valid envelope" });
      }
      const response = parsed.data;
      if (response.ok === false) {
        return err({ code: response.payload.code, message: response.payload.message });
      }
      if (response.id !== envelope.id) {
        return err({ code: "internal", message: "the answer does not match the request" });
      }
      return ok(response.payload as unknown as ProtocolResults[K]);
    },
  };
}
