// Destinations: choosing them, and planning against them (copy-conflicted, free space).

import { open } from "@tauri-apps/plugin-dialog";
import { api, historyApi, type OffloadArgs } from "./api";
import { markEarlierRun } from "./history";
import { S, filesFromPlan, running, saveSettings, sep, tokenContext } from "./state.svelte.ts";
import { resolveRoot } from "./tokens";

function offloadArgs(): OffloadArgs {
  const ctx = tokenContext();
  const { copyDest, bakDest, tokens } = S.settings;
  return {
    source: S.src!.path,
    copyTo: resolveRoot(copyDest!, tokens, ctx, sep),
    backupTo: bakDest ? resolveRoot(bakDest, tokens, ctx, sep) : null,
    md5: S.settings.md5,
    fanOut: S.settings.fanOut,
    includeSystemFiles: S.settings.includeSystemFiles,
    mhl: S.settings.mhl,
    md5List: S.settings.md5List && S.settings.md5,
    keepAwake: S.settings.keepAwake,
  };
}

let prepTimer: number | undefined;
let prepSeq = 0;

export function schedulePrepare() {
  clearTimeout(prepTimer);
  prepTimer = window.setTimeout(prepare, 250);
}

/** Plans against the real destinations: resolves copy-conflicted, checks space. */
export async function prepare(): Promise<boolean> {
  clearTimeout(prepTimer);
  if (!S.src || (!S.settings.copyDest && !S.resumeJournal) || running() || S.phase !== "ready") {
    S.dests = null;
    S.prepError = null;
    return false;
  }
  const seq = ++prepSeq;
  try {
    const p = S.resumeJournal ? await historyApi.prepareResume(S.resumeJournal) : await api.prepare(offloadArgs());
    S.currentJournal = p.journal;
    if (seq !== prepSeq) return false;
    S.dests = p.destinations;
    S.prepError = null;
    S.plan = p.plan;
    S.files = filesFromPlan(p.plan);
    if (S.resumeJournal && S.resumeDetail) markEarlierRun(S.files, S.resumeDetail);
  } catch (e) {
    if (seq !== prepSeq) return false;
    S.dests = null;
    S.prepError = String(e);
  }
  return !S.prepError;
}

export function setDest(w: "copy" | "bak", path: string) {
  const s = S.settings;
  if (w === "copy") s.copyDest = path;
  else s.bakDest = path;
  const key = w === "copy" ? "recentCopy" : "recentBak";
  s[key] = [path, ...s[key].filter((p) => p !== path)].slice(0, 5);
  saveSettings();
  S.dests = null;
  schedulePrepare();
}

/** Picks a destination folder with the system dialog, starting where the current one is. */
export async function browseDest(w: "copy" | "bak") {
  const s = S.settings;
  const current = (w === "copy" ? s.copyDest : s.bakDest) ?? undefined;
  const p = await open({ directory: true, title: w === "copy" ? "Copy media to…" : "Back up media to…", defaultPath: current });
  if (typeof p === "string") setDest(w, p);
}

/** Drops the backup destination: the next offload makes only the copy. */
export function removeBackup() {
  S.settings.bakDest = null;
  saveSettings();
  S.dests = null;
  schedulePrepare();
}

export function tokensChanged() {
  saveSettings();
  S.dests = null;
  schedulePrepare();
}
