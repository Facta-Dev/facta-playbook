// exampletool — the fake platform runtime.
//
// The messaging server talks to a RuntimePort, not to chrome.*. The fake keeps
// the registered responder so a test can answer a message itself: no test
// ever touches the extension API (PLAYBOOK §17, "platform adapters against a
// fake browser").

import {
  type MessageResponder,
  type PlatformSender,
  type RuntimePort,
} from "../../../src/platform/runtime";

export interface FakeRuntime extends RuntimePort {
  /** Answers a message the way the platform would. */
  readonly respond: (message: unknown, sender: PlatformSender) => Promise<unknown>;
  /** True once a responder was registered. */
  readonly registered: () => boolean;
}

export function createFakeRuntime(id = "test-extension-id"): FakeRuntime {
  let responder: MessageResponder | undefined;

  return {
    id,
    onMessage(next: MessageResponder) {
      responder = next;
    },
    sendMessage(): Promise<unknown> {
      return Promise.resolve(undefined);
    },
    respond: (message, sender) => {
      if (responder === undefined) throw new Error("no message listener was registered");
      return responder(message, sender);
    },
    registered: () => responder !== undefined,
  };
}

/** A sender with no fact set: every field is filled by the caller. */
export function createSender(overrides: Partial<PlatformSender> = {}): PlatformSender {
  return {
    id: undefined,
    url: undefined,
    origin: undefined,
    frameId: undefined,
    tabId: undefined,
    ...overrides,
  };
}
