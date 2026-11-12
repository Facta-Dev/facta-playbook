// <app> — the notes domain invariants.
//
// The rules of a note live in src/features/notes/domain/note.ts and nowhere
// else. These tests pin them: a title is short and plain, a body is bounded
// and plain, and a note that reaches the application always holds.

import { describe, expect, it } from "vitest";
import {
  createNote,
  validateBody,
  validateTitle,
  NOTE_BODY_MAX_LENGTH,
  NOTE_TITLE_MAX_LENGTH,
} from "../../../../../src/features/notes/domain/note";
import { expectError, expectOk } from "../../../../_shared/result";

describe("notes domain", () => {
  describe("validateTitle", () => {
    it("rejects an empty title", () => {
      expect(expectError(validateTitle("")).code).toBe("invalid-title");
    });

    it("rejects a title that is only whitespace", () => {
      expect(expectError(validateTitle("   \n\t ")).code).toBe("invalid-title");
    });

    it("trims the title it accepts", () => {
      expect(expectOk(validateTitle("  a title  "))).toBe("a title");
    });

    it("accepts a title at the maximum length", () => {
      const title = "a".repeat(NOTE_TITLE_MAX_LENGTH);
      expect(expectOk(validateTitle(title))).toBe(title);
    });

    it("rejects a title longer than the maximum length", () => {
      expect(expectError(validateTitle("a".repeat(NOTE_TITLE_MAX_LENGTH + 1))).code).toBe(
        "invalid-title",
      );
    });

    it("rejects HTML in the title", () => {
      expect(expectError(validateTitle("<b>a title</b>")).code).toBe("invalid-title");
    });

    it("rejects an HTML tag in the title", () => {
      expect(expectError(validateTitle("a title <img src=x>")).code).toBe("invalid-title");
    });
  });

  describe("validateBody", () => {
    it("accepts an empty body", () => {
      expect(expectOk(validateBody(""))).toBe("");
    });

    it("keeps the body it accepts", () => {
      expect(expectOk(validateBody("a body"))).toBe("a body");
    });

    it("accepts a body at the maximum length", () => {
      const body = "a".repeat(NOTE_BODY_MAX_LENGTH);
      expect(expectOk(validateBody(body))).toBe(body);
    });

    it("rejects a body longer than the maximum length", () => {
      expect(expectError(validateBody("a".repeat(NOTE_BODY_MAX_LENGTH + 1))).code).toBe(
        "invalid-body",
      );
    });

    it("rejects HTML in the body", () => {
      expect(expectError(validateBody("<script>alert(1)</script>")).code).toBe("invalid-body");
    });
  });

  describe("createNote", () => {
    const input = {
      id: "note-1",
      title: "  a title  ",
      body: "a body",
      createdAt: "2024-01-01T00:00:00.000Z",
      updatedAt: "2024-01-02T00:00:00.000Z",
    };

    it("builds a note with the trimmed title", () => {
      const note = expectOk(createNote(input));
      expect(note.title).toBe("a title");
      expect(note.body).toBe("a body");
      expect(note.id).toBe("note-1");
      expect(note.createdAt).toBe(input.createdAt);
      expect(note.updatedAt).toBe(input.updatedAt);
    });

    it("refuses a note with an invalid title", () => {
      expect(expectError(createNote({ ...input, title: "" })).code).toBe("invalid-title");
    });

    it("refuses a note with an invalid body", () => {
      expect(expectError(createNote({ ...input, body: "<b>x</b>" })).code).toBe("invalid-body");
    });
  });
});
