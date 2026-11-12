// <app> — branded identifiers.
//
// A brand keeps a raw string from being passed where an id of another kind
// is expected. It costs nothing at runtime and needs no dependency.

declare const brand: unique symbol;

/** A string that is known to be the id of `K`. */
export type Branded<K extends string> = string & { readonly [brand]: K };

export type NoteId = Branded<"NoteId">;
export type RequestId = Branded<"RequestId">;
export type CorrelationId = Branded<"CorrelationId">;

export const asNoteId = (raw: string): NoteId => raw as NoteId;
export const asRequestId = (raw: string): RequestId => raw as RequestId;
export const asCorrelationId = (raw: string): CorrelationId => raw as CorrelationId;
