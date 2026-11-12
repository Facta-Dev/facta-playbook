# Permissions Register

Every permission, host pattern and manifest capability the extension
declares, with the reason it exists, the review that authorises it and the
path that removes it. **A permission that is not in this register does not
belong in `src/manifest.ts`.**

The register is the evidence for the least-privilege posture. The evaluation
`manifest module keeps the least-privilege permission posture`
(`evals/contracts.test.mjs`) fails the build if `host_permissions` is not
empty or if a broad pattern such as `<all_urls>` appears.

## 1. Permissions

| Permission  | Granted API                                    | Why the extension needs it                                                                                                                               | What it exposes                                                                                     | Review trigger                              | Removal path                                                                                                           |
| ----------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `storage`   | `chrome.storage.local`, `.session`, `.managed` | The notes record (`notes:record`), the user settings (`policy:settings`) and the session mirror. There is no other durable store in a browser extension. | The data the extension stores — the notes and the settings. Nothing is read from another extension. | A new slice that needs to persist something | Delete the key, drop the store, remove the permission from `src/manifest.ts`; the store falls back to an in-memory map |
| `sidePanel` | `chrome.sidePanel`                             | The side panel is the extension's main surface; `setPanelBehavior({ openPanelOnActionClick: true })` opens it from the toolbar button.                   | The extension's own UI only; a web page cannot open it.                                             | The surface changes (popup, tab)            | Remove `side_panel` and `action` from the manifest, drop `src/platform/sidepanel.ts`, move the UI to the options page  |
| `alarms`    | `chrome.alarms`                                | Periodic work must re-wake the worker, which is terminated after ~30 s of inactivity.                                                                    | Nothing by itself.                                                                                  | A feature needs scheduled work              | Remove the permission and the alarm creation; schedule with a message from an open surface instead                     |

Not requested, on purpose: `scripting`, `debugger`, `identity`, `webRequest`,
`cookies`, `tabs`, `history`, `bookmarks`, `downloads`, `nativeMessaging`,
`clipboardRead`, `clipboardWrite`, `desktopCapture`.

## 2. Host patterns

| Pattern                 | Kind                        | Why                                                                                                                        | What it exposes                                             | Review trigger                      | Removal path                                                                                |
| ----------------------- | --------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `https://example.com/*` | `content_scripts[].matches` | The example content script (`entrypoints/content/example.content.ts`), which sets a dataset attribute and imports nothing. | Nothing: the script reads no page data and holds no secret. | A real content script replaces it   | Remove the `content_scripts` entry from `src/manifest.ts` and delete `entrypoints/content/` |
| —                       | `host_permissions`          | Empty on purpose: the extension talks to no site.                                                                          | —                                                           | A feature needs to talk to a server | —                                                                                           |

The pattern is a placeholder for a real one, not a recommendation:
`https://example.com/*` matches a site that exists. Replace it with your own
pattern, or drop the content script entirely.

## 3. Other manifest capabilities

| Key                        | State  | Consequence                                                                                                                                                                                                      |
| -------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `key`                      | Absent | The extension id is derived from the install path, so it differs per machine. The `SenderPolicy` uses `chrome.runtime.id` at runtime, so the allowlist stays correct — but the id is not stable across installs. |
| `storage.managed_schema`   | Absent | `managed-schema.json` ships but is not referenced, so `chrome.storage.managed` is empty in a deployed build. See the [Admin Guide](admin-guide.md).                                                              |
| `content_security_policy`  | Absent | The MV3 default applies: `script-src 'self'; object-src 'self'`. See [Security & CSP](10-security-and-csp.md).                                                                                                   |
| `incognito`                | Absent | The extension does not run in incognito unless the user allows it.                                                                                                                                               |
| `web_accessible_resources` | Absent | No extension file is reachable from a web page.                                                                                                                                                                  |
| `externally_connectable`   | Absent | No web page may message the extension.                                                                                                                                                                           |
| `minimum_chrome_version`   | `116`  | The oldest Chrome with the side panel API.                                                                                                                                                                       |

## 4. The review process

A permission change is a security change. The steps, in order:

1. **Write the register entry first.** What it grants, why the feature needs
   it, what it exposes, and how it is removed.
2. **Run the review.** The `permission-change` skill
   (`.claude/skills/permission-change/SKILL.md`) and the
   `manifest-permissions-reviewer` agent
   (`.claude/agents/client/manifest-permissions-reviewer.md`) check the
   change against this register.
3. **Change the manifest and the code.** `src/manifest.ts` is the only file
   to edit; the browser calls live in `src/platform/**`.
4. **Update the store listing copy.** The permission prompt the user sees is
   derived from the manifest; the listing text must match the new posture.
5. **Gate.** `make ci` and `pnpm run evals`.

## 5. The removal path

Every capability has a way out, and it is exercised the same way:

| Step                                           | What it proves                                  |
| ---------------------------------------------- | ----------------------------------------------- |
| Remove the declaration from `src/manifest.ts`  | The browser stops granting it                   |
| Delete the platform adapter that used it       | No dead code keeps a call to a removed API      |
| Delete the feature or the store that needed it | The data stops being written                    |
| Run `make ci`                                  | Nothing references the removed API              |
| Run `pnpm run evals`                           | The least-privilege posture test is still green |
| Update this register                           | The evidence matches the manifest               |

An extension that cannot describe how a permission would be removed is a
permission that should not be there.

## 6. See also

- [Manifest & Permission Engineering](06-manifest-and-permission-engineering.md)
- [Security & CSP](10-security-and-csp.md)
- [Admin Guide](admin-guide.md)
