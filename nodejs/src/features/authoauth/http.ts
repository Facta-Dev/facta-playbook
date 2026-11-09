// Features authoauth HTTP adapter: fastify handlers over the use-case, at
// reference /user/auth + /account/github parity. The google endpoint answers
// the standard JSON envelope (login shape); the github endpoints ALWAYS 302.
// App errors throw — the app-wide error handler answers the {detail, code}
// envelope; redirect flows carry the error text into the URL instead.

import type { FastifyReply, FastifyRequest } from "fastify";
import { z } from "zod";
import type { components } from "#gen/api/schema.js";
import { decode, requestIP, sendJSON } from "#platform/httpx.js";
import { setAuthCookie, type Identity } from "#platform/authx.js";
import { from, type AppError } from "#shared/apperr.js";
import { UseCase, frontendBase, linkSuccessURL } from "./usecase.js";
import { errAuthFailed, errNoCode, LinkStatePrefix, type Result } from "./domain.js";

type AuthTokens = components["schemas"]["AuthTokens"];

const googleBody = z.object({ credential: z.string().min(1) });

// errText prefers the human detail over the code-prefixed Error() for the
// redirect's error_description.
const errText = (e: AppError): string => e.detail;

// userAgent mirrors go http.Request.UserAgent(): "" when the header is absent.
const userAgent = (req: FastifyRequest): string => {
  const v = req.headers["user-agent"];
  return typeof v === "string" ? v : Array.isArray(v) ? (v[0] ?? "") : "";
};

// authoauthHttp builds the handler table (uc plus the cookie/token options).
export const authoauthHttp = (
  uc: UseCase,
  opts: { accessTtlMs: number; cookieName: string; cookieSecure: boolean; frontendSuccess: string },
) => {
  // tokenPair converts a use-case Result into the generated wire shape.
  const tokenPair = (res: Result): AuthTokens => ({
    access_token: res.accessToken,
    refresh_token: res.refreshToken,
    token_type: "bearer",
    expires_in: Math.round(opts.accessTtlMs / 1000),
  });

  // setCookie issues the access-token cookie when cookie mode is on.
  const setCookie = (reply: FastifyReply, token: string): void => {
    if (opts.cookieName !== "") {
      setAuthCookie(reply, opts.cookieName, token, opts.cookieSecure);
    }
  };

  return {
    // google: One-Tap credential in, login shape out.
    google: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const b = decode(googleBody, req.body);
      const res = await uc.google(b.credential, requestIP(req), userAgent(req));
      setCookie(reply, res.accessToken);
      sendJSON(reply, 200, tokenPair(res));
    },

    // githubLogin: 302 to GitHub's authorize URL when enabled; a disabled/
    // unconfigured provider answers the 403 envelope (operator problem, not a
    // user-facing redirect).
    githubLogin: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const state = queryGet(req, "state");
      const target = await uc.githubLoginURL(state);
      reply.redirect(target);
    },

    // githubCallback: link flow or login flow; EVERY outcome is a 302 (tokens
    // in the fragment on success). A disabled/unconfigured provider answers
    // the 403 envelope instead.
    githubCallback: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      await uc.githubGate();
      let errorCode = queryGet(req, "error");
      let description = queryGet(req, "error_description");
      // GitHub sends its own consent errors straight back to us.
      if (errorCode === "" && queryGet(req, "code") === "") {
        errorCode = errNoCode;
        description = "";
      }
      if (errorCode !== "") {
        redirectError(reply, opts.frontendSuccess, errorCode, description);
        return;
      }
      const state = queryGet(req, "state");
      const code = queryGet(req, "code");
      if (state.startsWith(LinkStatePrefix)) {
        try {
          await uc.githubLink(state.slice(LinkStatePrefix.length), code);
        } catch (err) {
          redirectError(reply, opts.frontendSuccess, errAuthFailed, errText(from(err)));
          return;
        }
        reply.redirect(linkSuccessURL(opts.frontendSuccess));
        return;
      }
      const res = await uc.githubLogin(code, requestIP(req), userAgent(req));
      const fragment = new URLSearchParams({
        access_token: res.accessToken,
        refresh_token: res.refreshToken,
      }).toString();
      reply.redirect(`${frontendBase(opts.frontendSuccess)}#${fragment}`);
    },

    // githubConnect: bearer-protected {url} whose state carries the link JWT
    // (reference /account/github/connect-url).
    githubConnect: async (req: FastifyRequest, reply: FastifyReply): Promise<void> => {
      const actor = req.identity as Identity;
      const url = await uc.githubConnectURL(actor.userId);
      sendJSON(reply, 200, { url });
    },
  };
};

// redirectError bounces to the frontend success URL with ?error= pairs.
const redirectError = (
  reply: FastifyReply,
  frontend: string,
  code: string,
  description: string,
): void => {
  const q = new URLSearchParams({ error: code, error_description: description });
  reply.redirect(`${frontendBase(frontend)}?${q.toString()}`);
};

// queryGet reads one query param: "" when absent (go q.Get parity).
const queryGet = (req: FastifyRequest, key: string): string => {
  const raw = (req.query as Record<string, unknown> | undefined)?.[key];
  if (Array.isArray(raw)) {
    return String(raw[0] ?? "");
  }
  return raw === undefined ? "" : String(raw);
};
