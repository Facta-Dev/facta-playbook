// Features-support: request/assertion helpers the step modules share
// (steps_http.go helpers parity).

import type { HarnessWorld, TokenPair } from "./world.js";

// pair is the remembered pair behind a step label, or a helpful error.
export const pair = (w: HarnessWorld, label: string): TokenPair => {
  const p = w.pairs.get(label);
  if (p === undefined) {
    throw new Error(
      `no logged-in session "${label}": log in with 'remember the pair as ${label}' first`,
    );
  }
  return p;
};

// expand substitutes {{name}} placeholders in paths and bodies.
export const expand = (w: HarnessWorld, s: string): string => {
  for (const [k, v] of w.vars) {
    s = s.split(`{{${k}}}`).join(v);
  }
  return s;
};

// doSend issues one request against the real server and records the
// exchange; undici's redirect:"manual" exposes the 302 for assertions.
export const doSend = async (
  w: HarnessWorld,
  method: string,
  path: string,
  body: string,
  bearer: string,
): Promise<void> => {
  const h = new Headers();
  if (body !== "") {
    h.set("Content-Type", "application/json");
  }
  if (bearer !== "") {
    h.set("Authorization", `Bearer ${bearer}`);
  }
  const resp = await fetch(w.app.base + expand(w, path), {
    method,
    headers: h,
    redirect: "manual",
    body: body === "" ? undefined : expand(w, body),
  });
  w.last = {
    status: resp.status,
    body: (await resp.text()).slice(0, 1 << 20),
    location: resp.headers.get("location") ?? "",
  };
};

// jsonMap decodes the last response body into an object ("" = empty).
export const jsonMap = (w: HarnessWorld): Record<string, unknown> => {
  if (w.last.body === "") {
    return {};
  }
  const v: unknown = JSON.parse(w.last.body);
  if (v === null || typeof v !== "object" || Array.isArray(v)) {
    throw new Error(`response body is not a JSON object: ${w.last.body}`);
  }
  return v as Record<string, unknown>;
};

// jsonArr decodes the last response body into an array.
export const jsonArr = (w: HarnessWorld): unknown[] => {
  if (w.last.body === "") {
    return [];
  }
  const v: unknown = JSON.parse(w.last.body);
  if (!Array.isArray(v)) {
    throw new Error(`response body is not a JSON array: ${w.last.body}`);
  }
  return v;
};

// sprint mirrors fmt.Sprint for JSON scalars: bare "900"/"true", strings raw.
export const sprint = (v: unknown): string => (typeof v === "string" ? v : String(v));
