// Entry point: wires the modules together and starts the app.

// In `vite dev` outside Tauri, fake the backend so the UI can be previewed in a browser.
// Must run before anything below talks to Tauri; compiled out of production builds.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  await import("./preview");
}

import { mount } from "svelte";
import { api } from "./api";
import App from "./components/App/App.svelte";
import { setDest } from "./destinations";
import { onEngine } from "./engineEvents";
import { refreshHistory } from "./history";
import { guardWindowClose, onReport, startOffload } from "./offload";
import { initReverify } from "./reverify.svelte.ts";
import { clearSource, pollVolumes, selectSource } from "./source";
import { S } from "./state.svelte.ts";
import { initThumbs } from "./thumbs";
import { initUpdates } from "./updates";

mount(App, { target: document.getElementById("app")! });
guardWindowClose();
api.onEngine(onEngine);
api.onReport(onReport);
initThumbs();
initReverify();
initUpdates();
void refreshHistory();
pollVolumes();

// Dev builds only: a handle for driving the app from automated end-to-end checks.
if (import.meta.env.DEV) Object.assign(window, { __offload: { S, selectSource, setDest, startOffload, clearSource } });
