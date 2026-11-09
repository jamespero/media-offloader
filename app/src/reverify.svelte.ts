// "Re-verify a Folder…": re-checks a copy against its ASC MHL history (or .md5 list).
// Read-only; the window updates as the check runs, and closing it stops a running check.

import { open } from "@tauri-apps/plugin-dialog";
import { reverifyApi, type ManifestSource, type ReverifyReport } from "./api";
import { closeOverlay, overlay, whenOverlayCloses } from "./components/Overlay/overlayState.svelte.ts";
import Reverify from "./components/Reverify/Reverify.svelte";
import { showLog } from "./dialogs";

export const R = $state({
  path: "",
  running: false,
  files: 0,
  bytes: 0,
  done: 0,
  checked: 0,
  failed: 0,
  current: "",
  source: null as ManifestSource | null,
  report: null as ReverifyReport | null,
  error: null as string | null,
});

export async function chooseAndReverify() {
  const p = await open({ directory: true, title: "Re-verify a folder" });
  if (typeof p !== "string") return;
  Object.assign(R, { path: p, running: true, files: 0, bytes: 0, done: 0, checked: 0, failed: 0, current: "", source: null, report: null, error: null });
  overlay(Reverify);
  whenOverlayCloses(() => {
    if (R.running) reverifyApi.cancel();
  });
  try {
    await reverifyApi.start(p);
  } catch (e) {
    R.running = false;
    R.error = String(e);
  }
}

/** True while a re-verify check is reading files. */
export const reverifyRunning = () => R.running;

export function initReverify() {
  reverifyApi.onEvent((e) => {
    switch (e.type) {
      case "started":
        R.files = e.files;
        R.bytes = e.bytes;
        R.source = e.source;
        break;
      case "progress":
        R.done = e.done;
        break;
      case "fileStarted":
        R.current = e.path;
        break;
      case "fileFinished":
        R.checked += 1;
        if (e.status.state !== "verified") R.failed += 1;
        break;
      case "finished":
        break;
    }
  });
  reverifyApi.onReport((r) => {
    R.running = false;
    R.report = r;
  });
  reverifyApi.onError((msg) => {
    R.running = false;
    R.error = msg;
  });
}

export function reverifyLog() {
  if (R.report?.logPath) {
    const path = R.report.logPath;
    closeOverlay();
    showLog(path);
  }
}
