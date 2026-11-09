// Features widgets is the example CRUD domain, owner-scoped: every read and
// write is filtered by the authenticated owner, and lifecycle changes are
// published to the realtime hub (widget.created / updated / deleted). This
// file is domain-pure: no transport, no persistence, no platform (guarded by
// eslint's domain-pure rule — the Go template's depguard analog).

import {
  E,
  CodeForbidden,
  CodeNotFound,
  CodeUnprocessable,
  AuthForbidden,
  NotFound,
  Validation,
  type AppError,
} from "#shared/apperr.js";

// Widget kinds (the DB CHECK constraint mirrors this openapi enum).
export const WidgetKinds = ["sprocket", "cog", "flange"] as const;
export type WidgetKind = (typeof WidgetKinds)[number];

// Widget is one owned row.
export interface Widget {
  id: string;
  name: string;
  kind: WidgetKind;
  ownerUserId: string;
  createdAt: Date;
}

export const ErrNotFound: AppError = E(CodeNotFound, NotFound, "widget not found");
export const ErrForbidden: AppError = E(CodeForbidden, AuthForbidden, "not your widget");
export const ErrValidation: AppError = E(CodeUnprocessable, Validation, "invalid request");
export const ErrUnknownKind: AppError = E(CodeUnprocessable, Validation, "unknown kind");
