// <app> — sender validation.
//
// PLAYBOOK §6.2: a message server validates the sender BEFORE it parses
// anything. A message that reaches a handler has been proven to come from
// this extension and from a context that is allowed to send it.

import { type SenderClass } from "../contracts/protocol";
import { type ErrorCode } from "../shared-kernel/errors";
import { err, ok, type Result } from "../shared-kernel/result";
import { matchesPattern } from "./matchers";

/** The sender facts the platform hands over. */
export interface SenderFacts {
  readonly id: string | undefined;
  readonly url: string | undefined;
  readonly frameId: number | undefined;
  readonly tabId: number | undefined;
}

/** What the extension accepts as a sender. */
export interface SenderPolicy {
  /** The id of this extension: the only sender id that is accepted. */
  readonly extensionId: string;
  /** The origin of the extension pages, "chrome-extension://<id>". */
  readonly extensionOrigin: string;
  /** The match patterns the content scripts are registered for. */
  readonly contentScriptPatterns: readonly string[];
}

export function classifySender(
  sender: SenderFacts,
  policy: SenderPolicy,
): Result<SenderClass, ErrorCode> {
  // Another extension, a web page or an unknown context: never trusted.
  if (sender.id !== policy.extensionId) return err("forbidden-sender");
  const url = sender.url;
  if (url === undefined) return err("forbidden-sender");
  // Only the top frame is expected; a frame message is never legitimate.
  if (sender.frameId !== undefined && sender.frameId !== 0) return err("forbidden-sender");
  // An extension page: the popup, the side panel or the options page.
  if (url.startsWith(`${policy.extensionOrigin}/`)) return ok("ui");
  // A content script: it runs in a tab, on a page the manifest allows.
  if (sender.tabId !== undefined) {
    const allowed = policy.contentScriptPatterns.some((pattern) => matchesPattern(pattern, url));
    return allowed ? ok("content") : err("forbidden-sender");
  }
  return err("forbidden-sender");
}
