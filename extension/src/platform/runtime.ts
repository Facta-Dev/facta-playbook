// <app> — the runtime transport.
//
// One of the files allowed to touch chrome.* (PLAYBOOK §3.1). Everything it
// hands out is a plain value: no chrome type crosses this folder's
// boundary.

/** The sender metadata the messaging layer is allowed to see. */
export interface PlatformSender {
  readonly id: string | undefined;
  readonly url: string | undefined;
  readonly origin: string | undefined;
  readonly frameId: number | undefined;
  readonly tabId: number | undefined;
}

/** Answers one message. */
export type MessageResponder = (message: unknown, sender: PlatformSender) => Promise<unknown>;

export interface RuntimePort {
  /** The id of this extension: the only sender id the server accepts. */
  readonly id: string;
  /** Registers the message listener. Called once, at the top level. */
  onMessage(responder: MessageResponder): void;
  /** Sends a message to the other extension contexts. */
  sendMessage(message: unknown): Promise<unknown>;
}

export function createRuntime(): RuntimePort {
  return {
    id: chrome.runtime.id,
    onMessage(responder) {
      chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
        void responder(message as unknown, toPlatformSender(sender))
          .then((response) => {
            sendResponse(response);
          })
          .catch(() => {
            // The server never rejects; this is the backstop that keeps a
            // broken responder from taking the worker down.
            sendResponse(null);
          });
        // The answer is asynchronous: keep the channel open.
        return true;
      });
    },
    async sendMessage(message) {
      return chrome.runtime.sendMessage<unknown, unknown>(message);
    },
  };
}

/** The match patterns the content scripts are registered for. */
export function contentScriptPatterns(): string[] {
  const manifest = chrome.runtime.getManifest();
  const patterns: string[] = [];
  for (const script of manifest.content_scripts ?? []) {
    for (const pattern of script.matches ?? []) {
      patterns.push(pattern);
    }
  }
  return patterns;
}

function toPlatformSender(sender: chrome.runtime.MessageSender): PlatformSender {
  return {
    id: sender.id,
    url: sender.url,
    origin: sender.origin,
    frameId: sender.frameId,
    tabId: sender.tab?.id,
  };
}
