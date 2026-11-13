// <app> — the note entity.
//
// The invariants of the domain, tested without a device, an emulator or a
// renderer: the entity is pure TypeScript, so a node test is enough.

import { describe, expect, it } from "vitest";
import {
  NOTE_BODY_MAX_LENGTH,
  NOTE_TITLE_MAX_LENGTH,
  createNote,
  validateBody,
  validateTitle,
} from "../../../../packages/feature-notes/src/domain/note";

describe("validateTitle", () => {
  it("accepts a title and trims it", () => {
    const result = validateTitle("  Shopping list  ");
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.value).toBe("Shopping list");
  });

  it("refuses an empty title", () => {
    const result = validateTitle("   ");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-title");
  });

  it("refuses a title longer than the maximum", () => {
    const result = validateTitle("x".repeat(NOTE_TITLE_MAX_LENGTH + 1));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-title");
  });

  it("refuses markup", () => {
    const result = validateTitle("<b>bold</b>");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-title");
  });
});

describe("validateBody", () => {
  it("accepts a body", () => {
    const result = validateBody("milk, bread");
    expect(result.ok).toBe(true);
  });

  it("refuses a body longer than the maximum", () => {
    const result = validateBody("x".repeat(NOTE_BODY_MAX_LENGTH + 1));
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-body");
  });

  it("refuses markup", () => {
    const result = validateBody("<script>alert(1)</script>");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-body");
  });
});

describe("createNote", () => {
  const valid = {
    id: "note_1",
    title: "Shopping list",
    body: "milk, bread",
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
  };

  it("builds a note from valid input", () => {
    const result = createNote(valid);
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.value.title).toBe("Shopping list");
  });

  it("refuses a note with an invalid title and never returns one", () => {
    const result = createNote({ ...valid, title: "" });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-title");
  });

  it("refuses a note with an invalid body", () => {
    const result = createNote({ ...valid, body: "<b>x</b>" });
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error.code).toBe("invalid-body");
  });
});
