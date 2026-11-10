// Opens the app's dialogs, reading whatever each one needs first.

import { getVersion } from "@tauri-apps/api/app";
import { api } from "./api";
import AboutDialog from "./components/Dialogs/AboutDialog.svelte";
import ConfirmStopDialog from "./components/Dialogs/ConfirmStopDialog.svelte";
import LogViewer from "./components/Dialogs/LogViewer.svelte";
import ReverifyIntroDialog from "./components/Dialogs/ReverifyIntroDialog.svelte";
import SavePresetDialog from "./components/Dialogs/SavePresetDialog.svelte";
import { overlay } from "./components/Overlay/overlayState.svelte.ts";
import { S } from "./state.svelte.ts";

export const confirmStop = () => overlay(ConfirmStopDialog);
export const savePresetDialog = () => overlay(SavePresetDialog);
export const reverifyIntro = () => overlay(ReverifyIntroDialog);

export async function showLog(path: string) {
  let text: string;
  try {
    text = await api.readText(path);
  } catch (e) {
    text = `Couldn't open the log: ${e}`;
  }
  overlay(LogViewer, { path, text });
}

/** The log of the offload on screen, once it has one. */
export function openRunLog() {
  if (S.report?.logPath) showLog(S.report.logPath);
}

/** The log of the past offload being viewed. */
export function openPastLog() {
  if (S.detail?.item.log) showLog(S.detail.item.log);
}

export async function about() {
  const version = await getVersion().catch(() => "unknown");
  overlay(AboutDialog, { version });
}
