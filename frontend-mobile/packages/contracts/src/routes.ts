// <app> — the route table.
//
// One table, one grammar, one parser. Screens never write a path by hand:
// they ask for a built path, and the same table parses what comes back.
//
// The grammar (PLAYBOOK_URLS.md, PLAYBOOK_FRONTEND_MOBILE.md §6):
//
//   /{userType}/{module}/{resource}
//   /{userType}/{module}/{resource}/{resourceId}
//   /{userType}/{module}/{resource}/{resourceId}/{subresource}
//
// plus the session-less routes of the public experience.
//
// The fifth segment (the subresource) is part of the shared grammar, but a
// RouteSpec declares no subresource: a path with five segments is
// grammatically valid and unknown to this table until a row declares one.
//
// The {userType} segment is an experience namespace, never an
// authorization: the server validates the session and the permissions.

import { err, ok, type Result } from "@<scope>/core";

/** The experience namespaces of the URL grammar. */
export const USER_TYPES = [
  "app",
  "admin",
  "owner",
  "manager",
  "agent",
  "partner",
  "public",
] as const;

/** An experience namespace. */
export type UserType = (typeof USER_TYPES)[number];

/** The routes of the session-less experience. */
export const SESSIONLESS_ROUTES = [
  "/onboarding",
  "/login",
  "/register",
  "/forgot-password",
  "/reset-password",
  "/verify-email",
] as const;

/** A route of the session-less experience. */
export type SessionlessRoute = (typeof SESSIONLESS_ROUTES)[number];

/** One entry of the route table. */
export interface RouteSpec {
  /** The experience namespace. */
  readonly userType: UserType;
  /** The module (the slice). */
  readonly module: string;
  /** The plural resource. */
  readonly resource: string;
  /** Whether the route accepts a resource id. */
  readonly byId: boolean;
}

/**
 * The route table.
 *
 * `notes/entries` is the example slice: the notes module and its entries.
 * Replace the rows with your own domains — the builders and the parser
 * follow the table, so nothing else has to change.
 */
export const ROUTE_TABLE: readonly RouteSpec[] = [
  { userType: "app", module: "notes", resource: "entries", byId: true },
  { userType: "app", module: "market", resource: "products", byId: true },
  { userType: "app", module: "notifications", resource: "preferences", byId: false },
  { userType: "admin", module: "tenants", resource: "users", byId: true },
];

/** A resource id accepted by the grammar: opaque, URL-safe, bounded. */
const RESOURCE_ID = /^[A-Za-z0-9_-]{1,128}$/;

/** The error of a path that does not match the grammar. */
export interface RouteError {
  readonly code: "unknown-route" | "invalid-path";
  readonly message: string;
}

/** What a parsed path is. */
export type ParsedRoute =
  | { readonly kind: "sessionless"; readonly route: SessionlessRoute }
  | {
      readonly kind: "resource";
      readonly spec: RouteSpec;
      readonly resourceId?: string | undefined;
    }
  | { readonly kind: "unknown"; readonly path: string };

/**
 * Builds a path from the grammar.
 *
 * The resource id is validated: an id that would change the shape of the
 * path is a programming error, not a URL.
 */
export function resourcePath(
  userType: UserType,
  module: string,
  resource: string,
  resourceId?: string,
): string {
  const base = `/${userType}/${module}/${resource}`;
  if (resourceId === undefined) return base;
  if (!RESOURCE_ID.test(resourceId)) {
    throw new Error(`resourcePath: invalid resource id '${resourceId}'`);
  }
  return `${base}/${resourceId}`;
}

/** The path of the notes collection. */
export function notesListPath(userType: UserType = "app"): string {
  return resourcePath(userType, "notes", "entries");
}

/** The path of one note. */
export function notePath(noteId: string, userType: UserType = "app"): string {
  return resourcePath(userType, "notes", "entries", noteId);
}

/** The path of a session-less route. */
export function sessionlessPath(route: SessionlessRoute): string {
  return route;
}

/** The route table row that matches a triple, if any. */
export function findRoute(
  userType: string,
  module: string,
  resource: string,
): RouteSpec | undefined {
  return ROUTE_TABLE.find(
    (spec) => spec.userType === userType && spec.module === module && spec.resource === resource,
  );
}

/**
 * Parses a path back into the route table.
 *
 * A path that is not in the table is not an error the app can fix: it is
 * `unknown`, and the caller decides what to do (the guard sends the user to
 * the login screen, the 403 screen or home).
 */
export function parseRoute(path: string): Result<ParsedRoute, RouteError> {
  const withoutQuery = path.split(/[?#]/)[0] ?? "";
  if (!withoutQuery.startsWith("/")) {
    return err({ code: "invalid-path", message: `'${path}' is not an absolute path.` });
  }

  const segments = withoutQuery.split("/").filter((segment) => segment.length > 0);

  if (segments.length === 1) {
    const candidate = `/${segments[0] ?? ""}` as SessionlessRoute;
    if ((SESSIONLESS_ROUTES as readonly string[]).includes(candidate)) {
      return ok({ kind: "sessionless", route: candidate });
    }
    return err({ code: "unknown-route", message: `'${path}' is not a known route.` });
  }

  if (segments.length !== 3 && segments.length !== 4 && segments.length !== 5) {
    return err({ code: "unknown-route", message: `'${path}' does not match the route grammar.` });
  }

  const [userType, module, resource, resourceId] = segments;
  const spec = findRoute(userType ?? "", module ?? "", resource ?? "");
  if (!spec) {
    return err({ code: "unknown-route", message: `'${path}' is not in the route table.` });
  }

  if (segments.length === 5) {
    return err({
      code: "unknown-route",
      message: `'${path}' has a subresource the route table does not declare.`,
    });
  }

  if (segments.length === 3) {
    return ok({ kind: "resource", spec });
  }

  if (!spec.byId) {
    return err({ code: "unknown-route", message: `'${path}' has an id the route does not take.` });
  }
  if (resourceId === undefined || !RESOURCE_ID.test(resourceId)) {
    return err({ code: "invalid-path", message: `'${path}' has an invalid resource id.` });
  }
  return ok({ kind: "resource", spec, resourceId });
}
