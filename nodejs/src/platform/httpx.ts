// Platform httpx holds the HTTP-edge helpers every slice shares: the error
// envelope, the zod trust-boundary decode (validator-tags analog), limit/
// offset pagination, and the client-IP extraction. Fastify owns transport;
// these helpers only shape it.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import { AppError, E, CodeUnprocessable, Validation, from } from "#shared/apperr.js";

// Envelope is the stable error shape.
export interface Envelope {
  detail: string;
  code: string;
}

export const envelope = (e: AppError): Envelope => ({ detail: e.detail, code: e.code });

// sendJSON writes v as JSON with the given status.
export const sendJSON = (reply: FastifyReply, status: number, value: unknown): void => {
  void reply.code(status).type("application/json").send(value);
};

// sendErr writes the {"detail","code"} envelope from any thrown value.
export const sendErr = (reply: FastifyReply, err: unknown): void => {
  const ae = from(err);
  sendJSON(reply, ae.http, envelope(ae));
};

// maxBodyBytes caps every JSON body at the edge (1 MiB) so no endpoint can
// allocate unbounded memory parsing a request.
export const maxBodyBytes = 1 << 20;

// Map a zod issue to the validator-tag error shape: "<field>: failed on '<tag>'".
const issueError = (error: z.ZodError): AppError => {
  const i0 = error.issues[0];
  if (!i0) return E(CodeUnprocessable, Validation, "invalid request");
  const field = i0.path.length > 0 ? i0.path.join(".") : "body";
  return E(CodeUnprocessable, Validation, `${field}: failed on '${i0.code}'`);
};

// decode parses the request body against a zod schema at the trust boundary.
export const decode = <T>(schema: z.ZodType<T>, body: unknown): T => {
  const result = schema.safeParse(body);
  if (!result.success) throw issueError(result.error);
  return result.data;
};

// requestIP returns the caller address: the RIGHTMOST hop of
// X-Forwarded-For — the proxy chain appends the real client at the edge
// (nginx $proxy_add_x_forwarded_for), so the last hop is the only
// client-unspoofable one — falling back to Fastify's request.ip.
export const requestIP = (req: FastifyRequest): string => {
  const fwd = req.headers["x-forwarded-for"];
  if (typeof fwd === "string" && fwd !== "") {
    const parts = fwd.split(",");
    return (parts.at(-1) ?? "").trim();
  }
  return req.ip;
};

// queryPage parses limit/offset with sane clamps (limit 1..100, offset >= 0),
// defaults 20/0.
export const queryPage = (req: FastifyRequest): { limit: number; offset: number } => {
  const q = req.query as Record<string, string | string[] | undefined>;
  const one = (name: string, err: string): string | undefined => {
    const v = q[name];
    if (v === undefined) return undefined;
    if (Array.isArray(v)) throw E(CodeUnprocessable, Validation, err);
    return v;
  };
  let limit = 20;
  const rawLimit = one("limit", "limit must be 1..100");
  if (rawLimit !== undefined) {
    const n = Number(rawLimit);
    if (!Number.isFinite(n) || !Number.isInteger(n) || n < 1 || n > 100) {
      throw E(CodeUnprocessable, Validation, "limit must be 1..100");
    }
    limit = n;
  }
  let offset = 0;
  const rawOffset = one("offset", "offset must be >= 0");
  if (rawOffset !== undefined) {
    const n = Number(rawOffset);
    if (!Number.isFinite(n) || !Number.isInteger(n) || n < 0) {
      throw E(CodeUnprocessable, Validation, "offset must be >= 0");
    }
    offset = n;
  }
  return { limit, offset };
};
