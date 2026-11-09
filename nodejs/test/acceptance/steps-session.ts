// Session steps (steps_authsession.go parity).

import { When } from "@cucumber/cucumber";
import { doSend, jsonMap, pair } from "./support/http.js";
import type { HarnessWorld } from "./support/world.js";

// loginOnce posts the login without asserting the status.
const loginOnce = (w: HarnessWorld, email: string, pw: string): Promise<void> =>
  doSend(w, "POST", "/api/v1/auth/login", JSON.stringify({ email, password: pw }), "");

When(
  /^I log in as "([^"]*)" with password "([^"]*)"$/,
  async function (this: HarnessWorld, email: string, pw: string) {
    await loginOnce(this, email, pw);
  },
);

When(
  /^I log in as "([^"]*)" with password "([^"]*)" (\d+) times$/,
  async function (this: HarnessWorld, email: string, pw: string, n: string) {
    for (let i = 0; i < Number(n); i += 1) {
      await loginOnce(this, email, pw);
    }
  },
);

When(
  /^I log in as "([^"]*)" with password "([^"]*)" and remember the pair as ([^:\s]+)$/,
  async function (this: HarnessWorld, email: string, pw: string, label: string) {
    await loginOnce(this, email, pw);
    if (this.last.status !== 200) {
      throw new Error(
        `login as ${email} failed with ${this.last.status} and body ${this.last.body}`,
      );
    }
    const m = jsonMap(this);
    const access = typeof m.access_token === "string" ? m.access_token : "";
    const refresh = typeof m.refresh_token === "string" ? m.refresh_token : "";
    if (access === "" || refresh === "") {
      throw new Error(`login response has no token pair: ${this.last.body}`);
    }
    this.pairs.set(label, { access, refresh });
  },
);

// Refresh does not overwrite the remembered pair: rotation means the old
// access token is what the scenario keeps asserting against.
When(/^I refresh using the token "([^"]+)"$/, async function (this: HarnessWorld, label: string) {
  const p = pair(this, label);
  await doSend(
    this,
    "POST",
    "/api/v1/auth/refresh",
    JSON.stringify({ refresh_token: p.refresh }),
    "",
  );
});

When(/^I log out with the token "([^"]+)"$/, async function (this: HarnessWorld, label: string) {
  const p = pair(this, label);
  await doSend(
    this,
    "POST",
    "/api/v1/auth/logout",
    JSON.stringify({ refresh_token: p.refresh }),
    p.access,
  );
});
