// Admin steps (steps_adminusers.go parity).

import { Then } from "@cucumber/cucumber";
import type { HarnessWorld } from "./support/world.js";

// The audit trail is in-memory shared state rather than an HTTP response, so
// it gets its own step.
Then(
  /^(\d+) audit entr(?:y|ies) (?:is|are|was|were) recorded$/,
  function (this: HarnessWorld, want: string) {
    if (this.app.table.audit.length !== Number(want)) {
      throw new Error(`want ${want} audit entries, got ${this.app.table.audit.length}`);
    }
  },
);
