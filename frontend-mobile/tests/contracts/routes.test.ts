// <app> — the route table.
//
// The builders and the parser are the same table seen from both sides: a
// built path always parses back to the route it came from.

import { describe, expect, it } from "vitest";
import {
  ROUTE_TABLE,
  SESSIONLESS_ROUTES,
  notePath,
  notesListPath,
  parseRoute,
  resourcePath,
  sessionlessPath,
} from "@<scope>/contracts";

describe("the route builders", () => {
  it("builds a collection path", () => {
    expect(notesListPath()).toBe("/app/notes/entries");
    expect(notesListPath("admin")).toBe("/admin/notes/entries");
  });

  it("builds a path with a resource id", () => {
    expect(notePath("note_1")).toBe("/app/notes/entries/note_1");
  });

  it("builds a path of the grammar", () => {
    expect(resourcePath("app", "market", "products", "p_1")).toBe("/app/market/products/p_1");
    expect(resourcePath("admin", "tenants", "users")).toBe("/admin/tenants/users");
  });

  it("refuses a resource id that would change the shape of the path", () => {
    expect(() => resourcePath("app", "notes", "entries", "a/b")).toThrow();
    expect(() => resourcePath("app", "notes", "entries", "")).toThrow();
  });

  it("builds the session-less paths", () => {
    for (const route of SESSIONLESS_ROUTES) {
      expect(sessionlessPath(route)).toBe(route);
    }
  });
});

describe("parseRoute", () => {
  it("parses a session-less route", () => {
    const result = parseRoute("/login");
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.kind).toBe("sessionless");
      if (result.value.kind === "sessionless") expect(result.value.route).toBe("/login");
    }
  });

  it("parses a collection path back to its route", () => {
    const result = parseRoute(notesListPath());
    expect(result.ok).toBe(true);
    if (result.ok && result.value.kind === "resource") {
      expect(result.value.spec.module).toBe("notes");
      expect(result.value.spec.resource).toBe("entries");
      expect(result.value.resourceId).toBeUndefined();
    }
  });

  it("parses a path with a resource id", () => {
    const result = parseRoute(notePath("note_1"));
    expect(result.ok).toBe(true);
    if (result.ok && result.value.kind === "resource") {
      expect(result.value.resourceId).toBe("note_1");
    }
  });

  it("ignores the query string", () => {
    const result = parseRoute("/app/notes/entries?page=2");
    expect(result.ok).toBe(true);
  });

  it("refuses a path that is not in the table", () => {
    const result = parseRoute("/app/unknown/entries");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("unknown-route");
  });

  it("refuses a path that does not match the grammar", () => {
    const result = parseRoute("/app/notes");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("unknown-route");
  });

  it("refuses a path with a subresource the table does not declare", () => {
    const result = parseRoute("/app/notes/entries/note_1/comments");
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe("unknown-route");
      expect(result.error.message).toContain("subresource");
    }
  });

  it("refuses a relative path", () => {
    const result = parseRoute("app/notes/entries");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-path");
  });

  it("refuses an id on a route that takes none", () => {
    const result = parseRoute("/app/notifications/preferences/1");
    expect(result.ok).toBe(false);
  });

  it("round-trips every route of the table", () => {
    for (const spec of ROUTE_TABLE) {
      const path = resourcePath(spec.userType, spec.module, spec.resource);
      const result = parseRoute(path);
      expect(result.ok).toBe(true);
    }
  });
});
