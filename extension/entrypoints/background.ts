// <app> — background service worker (composition root).
//
// MV3 starts the worker cold: every listener has to be registered while this
// module is first evaluated, so the registrations live at the top level and
// stay declarative. Domain logic does not live here — it belongs to
// src/platform/** and src/features/**.
import { createRuntime, contentScriptPatterns } from "../src/platform/runtime";
import { createStorage, createWriteThrough } from "../src/platform/storage";
import { setPanelBehavior } from "../src/platform/sidepanel";
import { registerMessageServer } from "../src/messaging/server";
import { createNotesHandlers } from "../src/messaging/notes";
import { createNotesStore } from "../src/storage/notes-store";
import { createLogger } from "../src/telemetry/logger";

const runtime = createRuntime();
const storage = createStorage();
const logger = createLogger();

// The durable area; every write is mirrored into the volatile one.
const local = createWriteThrough(storage.local, storage.session);
const store = createNotesStore({ area: local, logger });

registerMessageServer({
  runtime,
  logger,
  senderPolicy: {
    extensionId: runtime.id,
    extensionOrigin: `chrome-extension://${runtime.id}`,
    contentScriptPatterns: contentScriptPatterns(),
  },
  handlers: createNotesHandlers(store),
});

// Toolbar click opens the side panel (see the "action" entry of the manifest).
void setPanelBehavior({ openPanelOnActionClick: true });
