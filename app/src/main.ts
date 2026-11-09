// Entry point: wires the modules together and starts the app.

// In `vite dev` outside Tauri, fake the backend so the UI can be previewed in a browser.
// Must run before anything below talks to Tauri; compiled out of production builds.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  await import("./preview");
}

import { initInput } from "./input";
import { api } from "./api";
import { mountFooter } from "./components/Footer/footerState.svelte.ts";
import { setDest } from "./destinations";
import { onEngine } from "./engineEvents";
import { refreshHistory } from "./history";
import { guardWindowClose, onReport, startOffload } from "./offload";
import { $, render } from "./render";
import { initReverify } from "./reverify";
import { clearSource, pollVolumes, selectSource } from "./source";
import { S } from "./state";
import { initThumbInput } from "./thumbInput";
import { initThumbs } from "./thumbs";
import { initUpdates } from "./updates";

initInput();
initThumbInput();
guardWindowClose();
api.onEngine(onEngine);
api.onReport(onReport);
mountFooter($("app"));
initThumbs();
initReverify();
initUpdates();
render();
refreshHistory().then(render);
pollVolumes();

// Dev builds only: a handle for driving the app from automated end-to-end checks.
if (import.meta.env.DEV) Object.assign(window, { __offload: { S, selectSource, setDest, startOffload, clearSource } });
