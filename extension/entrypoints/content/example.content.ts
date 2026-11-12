// <app> — example content script.
//
// Content scripts run in an untrusted page: they import nothing, keep no
// secret and never act on a message without validating its sender first
// (PLAYBOOK §13.2). The script is wired to a single match pattern in
// src/manifest.ts.
document.documentElement.dataset.exampleContent = "ready";
