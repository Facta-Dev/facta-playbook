# Manifest & Permission Engineering

The manifest is the extension's public contract: it declares what the
browser will grant, what the user will see in the store listing and what the
extension may do to a page. The kit keeps it in **one file**,
`src/manifest.ts`, and builds it into `dist/manifest.json`.

## 1. One source of truth

```ts
const manifest: chrome.runtime.ManifestV3 = {
  manifest_version: 3,
  // name and description carry the template tokens; scripts/rename.sh
  // substitutes them before the first install.
  version: "0.1.0",
  minimum_chrome_version: "116",
  permissions: ["storage", "sidePanel", "alarms"],
  host_permissions: [],
  background: { service_worker: "background.js" },
  // action.default_title also carries the app token.
  action: { default_title: "<app token>" },
  side_panel: { default_path: "sidepanel/index.html" },
  options_page: "options/index.html",
  content_scripts: [
    {
      matches: ["https://example.com/*"],
      js: ["content/example.content.js"],
      run_at: "document_idle",
      all_frames: false,
      world: "ISOLATED",
    },
  ],
  default_locale: "en",
};
```

`vite.config.ts` passes this object to `webExtension({ manifest: () => manifest })`,
so the file you edit is the file the browser reads. Never edit the generated
`dist/manifest.json`.

## 2. The permission posture

| Declaration              | Value                                | Why it is there                                          |
| ------------------------ | ------------------------------------ | -------------------------------------------------------- |
| `permissions`            | `storage`                            | The notes record, the user policy, the session mirror    |
| `permissions`            | `sidePanel`                          | The main surface of the extension                        |
| `permissions`            | `alarms`                             | Periodic work that must survive the worker's lifetime    |
| `host_permissions`       | `[]`                                 | The extension talks to no site                           |
| `content_scripts`        | one pattern, `https://example.com/*` | The example content script; a placeholder for a real one |
| `minimum_chrome_version` | `116`                                | The oldest Chrome with the side panel API                |
| `background.type`        | absent (classic)                     | See [ADR 0002](adr/0002-classic-service-worker.md)       |

Full detail, with the review and removal path for each entry, is in the
[Permissions Register](permissions-register.md).

## 3. Adding a permission is a review

A permission bump changes what the store listing tells the user and what an
attacker gets if the extension is compromised. The kit makes that a
first-class step:

1. Write the entry in [permissions-register.md](permissions-register.md):
   what it grants, why the feature needs it, what it exposes.
2. Run the `permission-change` skill
   (`.claude/skills/permission-change/SKILL.md`) and the
   `manifest-permissions-reviewer` agent
   (`.claude/agents/client/manifest-permissions-reviewer.md`).
3. Update the manifest and the code that uses it.
4. Run `make ci` and the eval `manifest module keeps the least-privilege
permission posture`, which fails if `host_permissions` is not empty or if
   a broad pattern such as `<all_urls>` appears.

## 4. What the manifest deliberately does not declare

| Not declared               | Consequence                                                                                                                                                                                               |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `key`                      | The extension id is derived from the path at install time; it changes if the extension is loaded unpacked from a different directory, and it differs between the Chrome Web Store and a self-hosted build |
| `storage.managed_schema`   | The shipped `managed-schema.json` is **not** wired into the manifest, so `chrome.storage.managed` is empty in a deployed build — see the [Admin Guide](admin-guide.md) for the wiring step                |
| `content_security_policy`  | The extension runs under the MV3 default policy (`script-src 'self'; object-src 'self'`) — see [Security & CSP](10-security-and-csp.md)                                                                   |
| `incognito`                | The extension does not run in incognito unless the user allows it                                                                                                                                         |
| `web_accessible_resources` | No page resource is exposed to a web page                                                                                                                                                                 |
| `externally_connectable`   | No web page may message the extension                                                                                                                                                                     |

These are gaps of the template, not accidents: each one is a deliberate
omission that a project re-adds with a reason.

## 5. Worked example

The toolbar button opens the side panel. The manifest declares the button
(`action`) and the panel path (`side_panel`); the behaviour is set by the
worker at start-up:

```ts
// entrypoints/background.ts
void setPanelBehavior({ openPanelOnActionClick: true });
```

The manifest and the worker agree because `src/platform/sidepanel.ts` is the
only place that names `chrome.sidePanel`.

## Do / Don't

| Do                                           | Don't                                         |
| -------------------------------------------- | --------------------------------------------- |
| Add one permission with a register entry     | Add `host_permissions` to make a request work |
| Keep the manifest in `src/manifest.ts`       | Edit `dist/manifest.json`                     |
| Declare the narrowest match pattern          | Use `<all_urls>` for convenience              |
| Re-check the store listing copy after a bump | Ship a permission change without a review     |

## See also

- [Permissions Register](permissions-register.md)
- [Security & CSP](10-security-and-csp.md)
- [Admin Guide](admin-guide.md)
