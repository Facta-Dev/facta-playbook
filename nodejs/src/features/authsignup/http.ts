// Features authsignup HTTP adapter: fastify handlers over the use-case. The
// shared generated model (src/gen/api) types the wire shapes; zod re-states
// the openapi contract for decode-time validation.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, sendJSON } from "#platform/httpx.js";
import { UseCase } from "./usecase.js";

type MessageResponse = components["schemas"]["MessageResponse"];

const signupBody = z.object({
  email: z.email(),
  password: z.string().min(8),
  first_name: z.string().max(100).optional(),
  last_name: z.string().max(100).optional(),
  language: z.string().max(10).optional(),
  user_type: z.string().optional(),
});

const verifyBody = z.object({
  email: z.email(),
  code: z
    .string()
    .regex(/^[0-9]{6}$/)
    .max(8),
});

const emailBody = z.object({ email: z.email() });

// accepted is the uniform acknowledgement body.
const accepted = (msg: string): MessageResponse => ({ message: msg });

export const authsignupHttp = (uc: UseCase) => ({
  // signup: 201 + "signup successful" (re-signup of an unverified email
  // re-issues the same 201 — idempotent); unknown user_type is 422.
  signup: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(signupBody, req.body);
    await uc.signup({
      email: b.email,
      firstName: b.first_name ?? "",
      lastName: b.last_name ?? "",
      password: b.password,
      language: b.language ?? "",
      userType: b.user_type ?? "",
    });
    sendJSON(reply, 201, accepted("signup successful"));
  },

  // verifyEmail consumes a 6-digit code: 200, or 400 with the code-status
  // error (expired / used / invalid / brute-force-capped).
  verifyEmail: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(verifyBody, req.body);
    await uc.verifyEmail(b.email, b.code);
    sendJSON(reply, 200, accepted("email verified"));
  },

  // resend: always 200, even when the account does not exist (no leak).
  resend: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
    const b = decode(emailBody, req.body);
    await uc.resendVerification(b.email);
    sendJSON(reply, 200, accepted("verification code sent"));
  },
});
