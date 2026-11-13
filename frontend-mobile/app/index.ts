// <app> — the app entry.
//
// The entry is thin: it registers the composition root and does nothing
// else (PLAYBOOK_FRONTEND_MOBILE.md §4.1).

import { registerRootComponent } from "expo";
import App from "./App";

registerRootComponent(App);
