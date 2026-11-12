// <app> — options page script.
//
// The options page is a plain extension page: it lists the effective policy
// and nothing else. It is read-only, and it never touches chrome.* directly
// — the policy reader goes through the platform adapter.
import { createPolicyReader } from "../../src/policy/reader";
import { type EffectivePolicy } from "../../src/policy/resolver";
import { createStorage } from "../../src/platform/storage";
import { createLogger } from "../../src/telemetry/logger";

const root = document.getElementById("root");
if (!root) {
  throw new Error("options root element is missing");
}
// The check above holds for the whole page, but a narrowing does not cross a
// function boundary: the narrowed root gets its own name.
const container: HTMLElement = root;

const logger = createLogger();
const storage = createStorage();
const reader = createPolicyReader({
  local: storage.local,
  managed: {
    // An extension without an administrator schema has no managed storage.
    // The empty policy is the honest answer, not a failure.
    read: async () => {
      try {
        return await storage.managed.read();
      } catch {
        return {};
      }
    },
  },
  logger,
});

/** The settings the page lists, in the order it lists them. */
const SETTINGS = [
  { key: "allowedHosts", label: "Allowed hosts" },
  { key: "blockedHosts", label: "Blocked hosts" },
  { key: "featureFlags", label: "Feature flags" },
  { key: "killSwitch", label: "Kill switch" },
  { key: "allowUserSettings", label: "User settings allowed" },
] as const;

function format(value: unknown): string {
  if (Array.isArray(value)) return value.length === 0 ? "none" : value.join(", ");
  if (typeof value === "boolean") return value ? "yes" : "no";
  if (typeof value === "object" && value !== null) {
    const entries = Object.entries(value);
    if (entries.length === 0) return "none";
    return entries.map(([name, flag]) => `${name}=${String(flag)}`).join(", ");
  }
  return String(value);
}

function render(policy: EffectivePolicy): void {
  const heading = document.createElement("h1");
  heading.textContent = "Effective policy";

  const list = document.createElement("dl");
  for (const setting of SETTINGS) {
    const term = document.createElement("dt");
    term.textContent = policy.locked.includes(setting.key)
      ? `${setting.label} (set by the administrator)`
      : setting.label;
    const value = document.createElement("dd");
    value.textContent = format(policy.settings[setting.key]);
    list.append(term, value);
  }

  container.replaceChildren(heading, list);
}

const status = document.createElement("p");
status.textContent = "Loading the policy…";
container.replaceChildren(status);

void reader
  .read()
  .then((policy) => render(policy))
  .catch((error: unknown) => {
    logger.log("error", "the policy could not be read", { error: String(error) });
    status.textContent = "The policy could not be read.";
  });
