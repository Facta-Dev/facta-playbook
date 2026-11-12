// <app> — Chrome match patterns.
//
// The content script allowlist and the policy host list are match patterns
// ("https://*.example.com/*"). The patterns come from the manifest and from
// the administrator, never from a page, so the matcher can stay small.

const MATCH_PATTERN = /^(\*|https?):\/\/([^/]+)(\/.*)$/;

/** True when the url matches the Chrome match pattern. */
export function matchesPattern(pattern: string, url: string): boolean {
  const parsed = MATCH_PATTERN.exec(pattern);
  if (!parsed) return false;
  const scheme = parsed[1];
  const host = parsed[2];
  const path = parsed[3];
  if (scheme === undefined || host === undefined || path === undefined) return false;

  let target: URL;
  try {
    target = new URL(url);
  } catch {
    return false;
  }
  if (scheme !== "*" && `${scheme}:` !== target.protocol) return false;
  if (!matchesHost(host, target.hostname)) return false;
  const source = path.split("*").map(escapeRegExp).join(".*");
  return new RegExp(`^${source}$`).test(target.pathname + target.search);
}

function matchesHost(pattern: string, hostname: string): boolean {
  if (pattern === "*") return true;
  if (pattern.startsWith("*.")) {
    const base = pattern.slice(2);
    return hostname === base || hostname.endsWith(`.${base}`);
  }
  return hostname === pattern;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
