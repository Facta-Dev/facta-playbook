// Social sign-in steps (steps_authoauth.go parity). Provider toggles rebuild
// the app (the use case copies its config at registration); the Background's
// bearer pairs stay valid across a rebuild because the JWT secret is shared
// and bearer verification is stateless.

import { When } from "@cucumber/cucumber";
import { doSend } from "./support/http.js";
import { newAppWith, type OAuthFlags } from "./app.js";
import type { HarnessWorld } from "./support/world.js";

When(
  /^"([^"]*)" signs in with a Google credential$/,
  async function (this: HarnessWorld, email: string) {
    await doSend(this, "POST", "/api/v1/auth/google", JSON.stringify({ credential: email }), "");
  },
);

When(
  /^"([^"]*)" signs in with an invalid Google credential$/,
  async function (this: HarnessWorld, _email: string) {
    await doSend(this, "POST", "/api/v1/auth/google", `{"credential": "not-a-credential"}`, "");
  },
);

When(/^the google provider is not configured$/, async function (this: HarnessWorld) {
  await rebuildApp(this, (f) => {
    f.google = false;
  });
});

When(/^the github provider is disabled$/, async function (this: HarnessWorld) {
  await rebuildApp(this, (f) => {
    f.github = false;
  });
});

// rebuildApp stops the running server and starts a fresh app with the given
// flags applied on the default switches. State (table, emails) starts empty
// again, mirroring a restart with different configuration.
const rebuildApp = async (w: HarnessWorld, apply: (f: OAuthFlags) => void): Promise<void> => {
  const flags: OAuthFlags = { google: true, github: true };
  apply(flags);
  await w.app.stop();
  w.app = await newAppWith(flags);
};
