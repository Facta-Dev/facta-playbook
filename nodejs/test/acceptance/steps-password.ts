// Password steps (steps_authpassword.go parity).

import { When } from "@cucumber/cucumber";
import { doSend, pair } from "./support/http.js";
import { wrongCode } from "./steps-signup.js";
import type { HarnessWorld } from "./support/world.js";

// forgotOnce posts the reset request without asserting the status.
const forgotOnce = (w: HarnessWorld, email: string): Promise<void> =>
  doSend(w, "POST", "/api/v1/auth/forgot-password", JSON.stringify({ email }), "");

When(
  /^I ask to reset the password for "([^"]*)"$/,
  async function (this: HarnessWorld, email: string) {
    await forgotOnce(this, email);
  },
);

When(
  /^I asked to reset the password for "([^"]*)" (\d+) times$/,
  async function (this: HarnessWorld, email: string, n: string) {
    for (let i = 0; i < Number(n); i += 1) {
      await forgotOnce(this, email);
    }
  },
);

When(
  /^I requested a reset code for "([^"]*)"$/,
  async function (this: HarnessWorld, email: string) {
    await forgotOnce(this, email);
    if (this.last.status !== 200) {
      throw new Error(
        `reset request for ${email} failed with ${this.last.status} and body ${this.last.body}`,
      );
    }
  },
);

// "validate with latest/wrong" share the validate-code POST shape.
const validateWith = (resolve: (w: HarnessWorld, email: string) => Promise<string>) =>
  async function (this: HarnessWorld, email: string): Promise<void> {
    const code = await resolve(this, email);
    await doSend(this, "POST", "/api/v1/auth/validate-code", JSON.stringify({ email, code }), "");
  };

When(
  /^I validate "([^"]*)" with the latest reset code$/,
  validateWith(async (w, email) => {
    const code = w.app.emails.latestCode(email, "reset_password");
    if (code === null) {
      throw new Error(`no reset_password code was enqueued for ${email}`);
    }
    return code;
  }),
);

When(
  /^I validate "([^"]*)" with a wrong reset code$/,
  validateWith(async (w, email) => {
    const code = w.app.emails.latestCode(email, "reset_password");
    if (code === null) {
      throw new Error(`no reset_password code was enqueued for ${email}`);
    }
    return wrongCode(code);
  }),
);

When(
  /^I reset the password for "([^"]*)" using the latest reset code and password "([^"]*)"$/,
  async function (this: HarnessWorld, email: string, pw: string) {
    const code = latestResetCode(this, email);
    await doSend(
      this,
      "POST",
      "/api/v1/auth/reset-password",
      JSON.stringify({ email, code, new_password: pw }),
      "",
    );
  },
);

// The current password is wrong => 401; the success case keeps this caller's
// own session (jti) alive and revokes the others.
When(
  /^I change the password of ([^:\s]+) from "([^"]*)" to "([^"]*)"$/,
  async function (this: HarnessWorld, label: string, current: string, fresh: string) {
    const p = pair(this, label);
    await doSend(
      this,
      "POST",
      "/api/v1/auth/change-password",
      JSON.stringify({ current_password: current, new_password: fresh }),
      p.access,
    );
  },
);

// latestResetCode reads the newest reset_password code for the address.
const latestResetCode = (w: HarnessWorld, email: string): string => {
  const code = w.app.emails.latestCode(email, "reset_password");
  if (code === null) {
    throw new Error(`no reset_password code was enqueued for ${email}`);
  }
  return code;
};
