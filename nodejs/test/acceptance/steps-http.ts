// The generic request/assert steps shared by all features (steps_http.go).

import { Then, When } from "@cucumber/cucumber";
import { doSend, jsonArr, jsonMap, pair, sprint } from "./support/http.js";
import type { HarnessWorld } from "./support/world.js";

When(
  /^I send "([A-Z]+)" "([^"]*)" as ([^:\s]+)$/,
  async function (this: HarnessWorld, method: string, path: string, who: string) {
    const p = pair(this, who);
    await doSend(this, method, path, "", p.access);
  },
);

When(
  /^I send "([A-Z]+)" "([^"]*)" as ([^:\s]+) with body:$/,
  async function (this: HarnessWorld, method: string, path: string, who: string, body: string) {
    const p = pair(this, who);
    await doSend(this, method, path, body, p.access);
  },
);

When(
  /^I send "([A-Z]+)" "([^"]*)" with body:$/,
  async function (this: HarnessWorld, method: string, path: string, body: string) {
    await doSend(this, method, path, body, "");
  },
);

When(
  /^I send "([A-Z]+)" "([^"]*)"$/,
  async function (this: HarnessWorld, method: string, path: string) {
    await doSend(this, method, path, "", "");
  },
);

Then(/^the response status is (\d+)$/, function (this: HarnessWorld, want: string) {
  const got = this.last.status;
  if (got !== Number(want)) {
    throw new Error(`got status ${got} with body ${this.last.body}, want ${want}`);
  }
});

Then(/^the response code is "([^"]*)"$/, function (this: HarnessWorld, want: string) {
  const m = jsonMap(this);
  if (sprint(m.code) !== want) {
    throw new Error(
      `got code ${JSON.stringify(m.code)} (body ${this.last.body}), want ${JSON.stringify(want)}`,
    );
  }
});

Then(
  /^the response field "([^"]*)" is "([^"]*)"$/,
  function (this: HarnessWorld, field: string, want: string) {
    const m = jsonMap(this);
    if (!(field in m)) {
      throw new Error(`response has no field "${field}" (body ${this.last.body})`);
    }
    if (sprint(m[field]) !== want) {
      throw new Error(
        `field "${field}" is ${JSON.stringify(m[field])} (body ${this.last.body}), want ${JSON.stringify(want)}`,
      );
    }
  },
);

Then(
  /^the response has token_type "([^"]*)" and expires_in (\d+)$/,
  function (this: HarnessWorld, wantType: string, wantExp: number) {
    const m = jsonMap(this);
    if (sprint(m.token_type) !== wantType || Number(m.expires_in) !== wantExp) {
      throw new Error(`want token_type "${wantType}" expires_in ${wantExp}, got ${this.last.body}`);
    }
  },
);

Then(/^the response lists (\d+) items?$/, function (this: HarnessWorld, want: string) {
  const m = jsonMap(this);
  const items = Array.isArray(m.items) ? m.items : [];
  if (items.length !== Number(want)) {
    throw new Error(`want ${want} item(s), got ${items.length} (body ${this.last.body})`);
  }
});

// Bare-array responses (session lists, anomaly reports).
Then(/^the list has (\d+) entr(?:y|ies)$/, function (this: HarnessWorld, want: string) {
  const arr = jsonArr(this);
  if (arr.length !== Number(want)) {
    throw new Error(`want ${want} entries, got ${arr.length} (body ${this.last.body})`);
  }
});

Then(
  /^the response field "([^"]*)" is "([^"]*)" on entry (\d+)$/,
  function (this: HarnessWorld, field: string, want: string, idx: string) {
    const arr = jsonArr(this);
    const i = Number(idx);
    if (i >= arr.length) {
      throw errorOf(`want entry ${i}, list has ${arr.length} (body ${this.last.body})`);
    }
    const m = arr[i];
    if (m === null || typeof m !== "object" || Array.isArray(m)) {
      throw errorOf(`entry ${i} is not an object (body ${this.last.body})`);
    }
    const row = m as Record<string, unknown>;
    if (!(field in row)) {
      throw errorOf(`entry ${i} has no field "${field}" (body ${this.last.body})`);
    }
    if (sprint(row[field]) !== want) {
      throw errorOf(
        `entry ${i} field "${field}" is ${JSON.stringify(row[field])} (body ${this.last.body}), want ${JSON.stringify(want)}`,
      );
    }
  },
);

Then(
  /^the response redirects to a URL that starts with "([^"]*)"$/,
  function (this: HarnessWorld, prefix: string) {
    const loc = this.last.location;
    if (!loc.startsWith(prefix)) {
      throw errorOf(
        `redirect ${JSON.stringify(loc)} does not start with ${JSON.stringify(prefix)}`,
      );
    }
  },
);

Then(/^the response redirects carrying an access token$/, function (this: HarnessWorld) {
  const loc = this.last.location;
  if (!loc.includes("access_token=") || !loc.includes("refresh_token=")) {
    throw errorOf(`redirect ${JSON.stringify(loc)} carries no token fragment`);
  }
});

Then(
  /^the response redirects carrying error "([^"]*)"$/,
  function (this: HarnessWorld, code: string) {
    const loc = this.last.location;
    if (!loc.includes(`error=${code}`)) {
      throw errorOf(
        `redirect ${JSON.stringify(loc)} carries error other than ${JSON.stringify(code)}`,
      );
    }
  },
);

Then(
  /^(\d+) "(verify_email|reset_password|password_changed)" emails? (?:is|are|was|were) enqueued for "([^"]*)"$/,
  function (this: HarnessWorld, want: string, template: string, to: string) {
    const got = this.app.emails.countOf(template, to);
    if (got !== Number(want)) {
      throw errorOf(`want ${want} "${template}" email(s) for ${to}, got ${got}`);
    }
  },
);

Then(
  /^I store the response field "([^"]*)" as ([^:\s]+)$/,
  function (this: HarnessWorld, field: string, name: string) {
    const m = jsonMap(this);
    if (!(field in m)) {
      throw errorOf(`response has no field "${field}" (body ${this.last.body})`);
    }
    this.vars.set(name, sprint(m[field]));
  },
);

Then(/^(\d+) hours pass$/, function (this: HarnessWorld, n: string) {
  this.app.clock.advance(Number(n), "h");
});

Then(/^(\d+) minutes pass$/, function (this: HarnessWorld, n: string) {
  this.app.clock.advance(Number(n), "min");
});

// errorOf keeps failure messages one-line (cucumber shows the step error).
const errorOf = (msg: string): Error => new Error(msg);
