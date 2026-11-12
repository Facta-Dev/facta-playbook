// exampletool — the sender allowlist.
//
// PLAYBOOK §6.2 and §13.2: a message server validates the sender before it
// parses anything. The allowlist accepts the extension pages of this
// extension and the content scripts of the hosts the manifest allows; every
// other sender is refused. The classifier is pure, so it is checked here
// without a browser.

import { describe, expect, it } from "vitest";
import { classifySender, type SenderPolicy } from "../../../src/security/sender";
import { expectError, expectOk } from "../../_shared/result";

const EXTENSION_ID = "test-extension-id";

const POLICY: SenderPolicy = {
  extensionId: EXTENSION_ID,
  extensionOrigin: `chrome-extension://${EXTENSION_ID}`,
  contentScriptPatterns: ["https://example.com/*"],
};

describe("sender allowlist", () => {
  it("accepts a page of this extension", () => {
    const result = classifySender(
      {
        id: EXTENSION_ID,
        url: `chrome-extension://${EXTENSION_ID}/sidepanel.html`,
        frameId: 0,
        tabId: undefined,
      },
      POLICY,
    );

    expect(expectOk(result)).toBe("ui");
  });

  it("accepts a content script on an allowed host", () => {
    const result = classifySender(
      { id: EXTENSION_ID, url: "https://example.com/page", frameId: 0, tabId: 7 },
      POLICY,
    );

    expect(expectOk(result)).toBe("content");
  });

  it("denies an origin that only looks like the extension origin", () => {
    const result = classifySender(
      {
        id: EXTENSION_ID,
        url: `chrome-extension://${EXTENSION_ID}-evil/sidepanel.html`,
        frameId: 0,
        tabId: undefined,
      },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies the extension origin without its trailing slash", () => {
    const result = classifySender(
      { id: EXTENSION_ID, url: `chrome-extension://${EXTENSION_ID}`, frameId: 0, tabId: undefined },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies an unknown external origin", () => {
    const result = classifySender(
      { id: EXTENSION_ID, url: "https://attacker.example/page", frameId: 0, tabId: undefined },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies a content script on a host the manifest does not allow", () => {
    const result = classifySender(
      { id: EXTENSION_ID, url: "https://attacker.example/page", frameId: 0, tabId: 7 },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies a sender that is not this extension", () => {
    const result = classifySender(
      {
        id: "another-extension",
        url: `chrome-extension://${EXTENSION_ID}/sidepanel.html`,
        frameId: 0,
        tabId: undefined,
      },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies a message from a sub frame", () => {
    const result = classifySender(
      {
        id: EXTENSION_ID,
        url: "https://example.com/page",
        frameId: 3,
        tabId: 7,
      },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });

  it("denies a sender without a url", () => {
    const result = classifySender(
      { id: EXTENSION_ID, url: undefined, frameId: 0, tabId: undefined },
      POLICY,
    );

    expect(expectError(result)).toBe("forbidden-sender");
  });
});
