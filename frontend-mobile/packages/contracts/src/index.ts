// <app> — the contracts barrel.
//
// DTO schemas and the route table. No React, no React Native, no Expo.

export {
  CreateNoteInput,
  NOTE_BODY_MAX_LENGTH,
  NOTE_TITLE_MAX_LENGTH,
  NoteDto,
  NoteListDto,
} from "./notes";

export {
  ROUTE_TABLE,
  SESSIONLESS_ROUTES,
  USER_TYPES,
  findRoute,
  notePath,
  notesListPath,
  parseRoute,
  resourcePath,
  sessionlessPath,
} from "./routes";
export type { ParsedRoute, RouteError, RouteSpec, SessionlessRoute, UserType } from "./routes";
