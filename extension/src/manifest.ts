// <app> — the extension manifest. The build writes it to dist/manifest.json;
// never edit the generated file, edit this one.
//
// Permission posture (PLAYBOOK §5.2): storage, side panel and alarms only,
// and no host permissions. Any new permission needs an entry in
// docs/permissions-register.md first — a permission bump is a security
// review, not a one-line change.
const manifest: chrome.runtime.ManifestV3 = {
  manifest_version: 3,
  name: "<app>",
  version: "0.1.0",
  description: "<app> — Chrome extension.",
  minimum_chrome_version: "116",
  permissions: ["storage", "sidePanel", "alarms"],
  // No blanket host grant: customer sites are granted per use case.
  host_permissions: [],
  background: {
    // Classic service worker — no "type": "module".
    service_worker: "background.js",
  },
  // Toolbar click opens the side panel; entrypoints/background.ts sets
  // setPanelBehavior({ openPanelOnActionClick: true }).
  action: {
    default_title: "<app>",
  },
  side_panel: {
    default_path: "sidepanel/index.html",
  },
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

export default manifest;
