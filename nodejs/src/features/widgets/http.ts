// Features widgets HTTP adapter: fastify handlers over the use-case. The
// shared generated model (src/gen/api, openapi-typescript) types the wire
// shapes — the Go template's oapi-codegen analog.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, queryPage, sendJSON } from "#platform/httpx.js";
import { roomUser } from "#platform/realtime/index.js";
import { ErrUnknownKind, WidgetKinds, type Widget, type WidgetKind } from "./domain.js";
import { UseCase } from "./usecase.js";
import type { Publisher } from "./ports.js";

type WidgetModel = components["schemas"]["Widget"];
type WidgetListModel = components["schemas"]["WidgetList"];

// Name validation sits in the schema (min 1 / max 120, like the openapi
// contract); the kind is checked against the domain's fixed vocabulary so a
// bad value answers the stable "unknown kind" error.
const widgetCreate = z.object({
  name: z.string().min(1).max(120),
  kind: z.string(),
});

// widgetModel maps a slice Widget onto the generated model.
const widgetModel = (w: Widget): WidgetModel => ({
  id: w.id,
  name: w.name,
  kind: w.kind,
  owner_user_id: w.ownerUserId,
  created_at: w.createdAt.toISOString(),
});

const identityOf = (req: FastifyRequest): string => req.identity?.userId ?? "";

// readCreate decodes + validates the shared WidgetCreate body (POST and PUT).
const readCreate = (req: FastifyRequest): { name: string; kind: WidgetKind } => {
  const body = decode(widgetCreate, req.body);
  const kind = body.kind as WidgetKind;
  if (!WidgetKinds.includes(kind)) throw ErrUnknownKind;
  return { name: body.name, kind };
};

export const widgetHttp = (uc: UseCase, pub: Publisher | null) => ({
  list: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const { limit, offset } = queryPage(req);
    const rows = await uc.list(identityOf(req), limit, offset);
    sendJSON(reply, 200, { items: rows.map(widgetModel) } satisfies WidgetListModel);
  },

  create: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const body = readCreate(req);
    const row = await uc.create(identityOf(req), body.name, body.kind);
    sendJSON(reply, 201, widgetModel(row));
  },

  get: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const row = await uc.get((req.params as { id: string }).id, identityOf(req));
    sendJSON(reply, 200, widgetModel(row));
  },

  update: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const body = readCreate(req);
    const row = await uc.update(
      (req.params as { id: string }).id,
      identityOf(req),
      body.name,
      body.kind,
    );
    sendJSON(reply, 200, widgetModel(row));
  },

  remove: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    await uc.delete((req.params as { id: string }).id, identityOf(req));
    await reply.code(204).send();
  },

  // events streams widget lifecycle events for the caller (SSE).
  events: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    if (pub === null) return; // hub-less wiring: nothing to stream
    reply.hijack();
    const res = reply.raw;
    res.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    });
    // SSE writes are best-effort: a client disconnect is the normal exit.
    res.write("retry: 2000\n\n");
    const sub = pub.subscribe(roomUser(identityOf(req)));
    res.on("close", () => sub.cancel());
    try {
      for await (const ev of sub) {
        res.write(`event: ${ev.type}\ndata: ${JSON.stringify(ev.payload)}\n\n`);
      }
    } catch {
      // ignore write-after-close and iterator teardown
    }
    res.end();
  },
});
