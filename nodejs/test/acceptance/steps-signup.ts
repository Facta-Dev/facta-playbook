// Signup steps (steps_authsignup.go parity).

import { Given, When } from "@cucumber/cucumber";
import { doSend } from "./support/http.js";
import type { HarnessWorld } from "./support/world.js";

When(
  /^I sign up "([^"]*)" with password "([^"]*)"$/,
  async function (this: HarnessWorld, email: string, pw: string) {
    await doSend(this, "POST", "/api/v1/auth/signup", signupBody(email, pw), "");
  },
);

Given(
  /^"([^"]*)" has signed up but not verified$/,
  async function (this: HarnessWorld, email: string) {
    // Seeding via the API keeps this step on the real path; the status is
    // not asserted (the feature asserts what follows).
    await doSend(this, "POST", "/api/v1/auth/signup", signupBody(email, "s3cure-pass-123"), "");
  },
);

// The unified seed step: "a verified user", "an unverified user", "an
// inactive user", "a blocked user", "an ADMIN user" — optional modifier,
// article stays grammatical in the feature text.
Given(
  /^an? (ADMIN |verified |unverified |inactive |blocked )?user "([^"]*)" with password "([^"]*)"$/,
  async function (this: HarnessWorld, mod: string | undefined, email: string, pw: string) {
    await this.app.table.seedUser(email, pw, (r) => {
      switch ((mod ?? "").trim()) {
        case "":
        case "verified":
          r.isVerified = true;
          break;
        case "unverified":
          break; // stays unverified (the default)
        case "inactive":
          r.isActive = false;
          r.isVerified = true;
          break;
        case "blocked":
          r.isBlocked = true;
          r.isVerified = true;
          break;
        case "ADMIN":
          r.isVerified = true;
          r.role = "ADMIN";
          break;
      }
    });
  },
);

When(
  /^I verify "([^"]*)" with the latest emailed code$/,
  async function (this: HarnessWorld, email: string) {
    const code = this.app.emails.latestCode(email, "verify_email");
    if (code === null) {
      throw new Error(`no verify_email code was enqueued for ${email}`);
    }
    await doSend(
      this,
      "POST",
      "/api/v1/auth/verify",
      `{"email": "${email}", "code": "${code}"}`,
      "",
    );
  },
);

When(/^I verify "([^"]*)" with a wrong code$/, async function (this: HarnessWorld, email: string) {
  const code = this.app.emails.latestCode(email, "verify_email");
  if (code === null) {
    throw new Error(`no verify_email code was enqueued for ${email}`);
  }
  await doSend(
    this,
    "POST",
    "/api/v1/auth/verify",
    `{"email": "${email}", "code": "${wrongCode(code)}"}`,
    "",
  );
});

When(/^I resend verification for "([^"]*)"$/, async function (this: HarnessWorld, email: string) {
  await doSend(this, "POST", "/api/v1/auth/verify/resend", `{"email": "${email}"}`, "");
});

// signupBody builds the signup request body.
const signupBody = (email: string, pw: string): string => JSON.stringify({ email, password: pw });

// wrongCode changes one digit of the code: same length, definitely a
// different value.
export const wrongCode = (code: string): string =>
  code.startsWith("0") ? `1${code.slice(1)}` : `0${code.slice(1)}`;
