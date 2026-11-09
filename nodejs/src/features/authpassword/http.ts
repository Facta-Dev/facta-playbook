// Features authpassword HTTP adapter: fastify handlers over the use-case.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, sendJSON } from "#platform/httpx.js";
import type { Identity } from "#platform/authx.js";
import { UseCase } from "./usecase.js";

type MessageResponse = components["schemas"]["MessageResponse"];

// ok is the uniform 200 acknowledgement.
const ok = (msg: string): MessageResponse => ({ message: msg });

const emailBody = z.object({ email: z.email() });
const validateBody = z.object({
  email: z.email(),
  code: z
    .string()
    .regex(/^[0-9]{6}$/)
    .max(8),
});
const resetBody = z.object({
  email: z.email(),
  code: z
    .string()
    .regex(/^[0-9]{6}$/)
    .max(8),
  new_password: z.string().min(8),
});
const changeBody = z.object({
  current_password: z.string().min(1),
  new_password: z.string().min(8),
});

export const authpasswordHttp = (uc: UseCase) => ({
  // forgot: always 200 (no existence leak); rate-limited per email.
  forgot: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(emailBody, req.body);
    await uc.forgot(b.email);
    sendJSON(reply, 200, ok("if the account exists, a reset code was sent"));
  },

  // validate checks a code without consuming it (200, or 400 code-status).
  validate: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(validateBody, req.body);
    await uc.validate(b.email, b.code);
    sendJSON(reply, 200, ok("code is valid"));
  },

  // reset consumes the code, sets the password, auto-verifies.
  reset: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(resetBody, req.body);
    await uc.reset(b.email, b.code, b.new_password);
    sendJSON(reply, 200, ok("password updated"));
  },

  // change: bearer-protected; jti (== sessions.id) is what survives
  // revocation of every other session.
  change: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(changeBody, req.body);
    const id = req.identity as Identity;
    await uc.change(id.userId, id.jti, b.current_password, b.new_password);
    sendJSON(reply, 200, ok("password updated"));
  },
});
