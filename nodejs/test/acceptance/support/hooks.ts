// Features-support: per-scenario lifecycle — a fresh app per scenario with a
// restarted id sequence (acceptance_test.go parity).

import { After, Before } from "@cucumber/cucumber";
import { newAppWith } from "../app.js";
import { resetIDs } from "../../testutil.js";
import type { HarnessWorld } from "./world.js";

Before(async function (this: HarnessWorld) {
  resetIDs();
  this.app = await newAppWith({ google: true, github: true });
});

After(async function (this: HarnessWorld) {
  // Close the SSE readers first: app.close blocks until handlers exit, and
  // the readers hold the connections open.
  for (const s of this.sse.values()) {
    s.close();
  }
  await this.app.stop();
});
