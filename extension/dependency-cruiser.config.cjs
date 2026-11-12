// <app> — dependency-cruiser architecture rules.
//
// The rules encode the layer boundaries of PLAYBOOK §3.1 and §4:
//
//   platform  -> owns chrome.* (the only place that touches the API)
//   feature   -> domain (pure) <- application <- adapters
//   feature   -> never imports another feature
//   sdk       -> no React, no chrome (it ships to every context)
//
// The chrome.* global itself is guarded by ESLint (no-restricted-globals);
// these rules guard the module level.
module.exports = {
  forbidden: [
    {
      name: "no-chrome-outside-platform",
      severity: "error",
      from: { pathNot: "^src/platform(/|$)" },
      to: { path: "(^|/)(chrome|webextension-polyfill)(/|$)" },
    },
    {
      name: "no-domain-to-application",
      severity: "error",
      from: { path: "^src/features/[^/]+/domain(/|$)" },
      to: { path: "^src/features/[^/]+/application(/|$)" },
    },
    {
      name: "no-application-to-adapters",
      severity: "error",
      from: { path: "^src/features/[^/]+/application(/|$)" },
      to: { path: "^src/features/[^/]+/adapters(/|$)" },
    },
    {
      name: "no-cross-feature-imports",
      severity: "error",
      from: { path: "^src/features/([^/]+)/" },
      to: { path: "^src/features/([^/]+)/", pathNot: "^src/features/$1/" },
    },
    {
      name: "no-sdk-react-or-chrome",
      severity: "error",
      from: { path: "^(sdk|packages/sdk)/" },
      to: { path: "(^|/)(react|react-dom|chrome|webextension-polyfill)(/|$)" },
    },
    {
      name: "no-circular",
      severity: "error",
      from: {},
      to: { circular: true },
    },
  ],
  options: {
    tsPreCompilationDeps: true,
    doNotFollow: { path: "node_modules" },
  },
};
