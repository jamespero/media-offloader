// The source: selecting a card or folder, resuming an earlier offload, ejecting, and
// watching for cards coming and going.

import { api, historyApi, type VolumeEntry } from "./api";
import { closeOverlay } from "./components/Overlay/overlayState.svelte.ts";
import { toast } from "./components/Toast/toastState.svelte.ts";
import { fmtSize } from "./format";
import { prepare, schedulePrepare } from "./destinations";
import { resetRun } from "./runState";
import { S, filesFromPlan, running, type Source } from "./state.svelte.ts";
import { thumbsForCurrent } from "./thumbRequests";

let selectSeq = 0;

function volumeMedia(v: VolumeEntry) {
  return `${v.fsType || "Drive"} · ${fmtSize(v.total)}`;
}

export async function selectSource(path: string, hint?: VolumeEntry) {
  if (running()) return;
  const seq = ++selectSeq;
  S.resumeJournal = null;
  S.currentJournal = null;
  S.view = "";
  S.kind = null;
  S.src = null;
  S.plan = null;
  S.files = [];
  S.dests = null;
  S.prepError = null;
  S.phase = "waiting";
  S.scanning = true;
  resetRun();
  try {
    const plan = await api.previewSource(path, S.settings.includeSystemFiles);
    if (seq !== selectSeq) return;
    const vol = hint ?? S.volumes.find((v) => samePath(v.path, plan.source.root));
    const kind: Source["kind"] = plan.source.kind === "folder" ? "folder" : vol?.removable ? "card" : "drive";
    S.src = { path, kind, label: plan.source.label, media: kind === "folder" ? "Folder" : vol ? volumeMedia(vol) : "Drive" };
    S.plan = plan;
    S.files = filesFromPlan(plan);
    S.phase = "ready";
  } catch (e) {
    if (seq !== selectSeq) return;
    toast(false, "Couldn't read the source", String(e));
  }
  S.scanning = false;
  schedulePrepare();
  thumbsForCurrent();
}

function samePath(a: string, b: string) {
  const n = (p: string) => p.replace(/[\\/]+$/, "").toLowerCase();
  return n(a) === n(b);
}

/** Sets up the Current tab to continue an earlier offload (or retry its failed files). */
export async function resumeFrom(path: string) {
  if (running()) return;
  closeOverlay();
  const seq = ++selectSeq;
  S.resumeDetail = null;
  try {
    S.resumeDetail = await historyApi.detail(path);
  } catch (e) {
    toast(false, "Couldn't read that offload", String(e));
    return;
  }
  if (seq !== selectSeq) return;
  const it = S.resumeDetail.item;
  const vol = S.volumes.find((v) => samePath(v.path, it.source));
  resetRun();
  S.view = "";
  S.resumeJournal = path;
  S.src = {
    path: it.source,
    label: it.label,
    kind: it.sourceKind === "folder" ? "folder" : vol?.removable === false ? "drive" : "card",
    media: it.sourceKind === "folder" ? "Folder" : vol ? volumeMedia(vol) : "Drive",
  };
  S.plan = null;
  S.files = [];
  S.dests = null;
  S.prepError = null;
  S.phase = "ready";
  S.scanning = true;
  await prepare();
  S.scanning = false;
  thumbsForCurrent();
}

/** Ejects the card (not while it's being read). */
export async function ejectSource() {
  const src = S.src;
  if (!src || src.kind === "folder" || S.ejected) return;
  if (S.phase === "copying" || (S.phase === "backing" && S.runFanOut)) return;
  try {
    await api.eject(src.path);
    toast(true, `${src.label} ejected`, "Safe to remove.");
    if (S.phase === "ready" || S.phase === "waiting") {
      clearSource();
      return;
    }
    S.ejected = true;
  } catch (e) {
    toast(false, `Couldn't eject ${src.label}`, String(e));
  }
}

export function clearSource() {
  selectSeq++;
  S.resumeJournal = null;
  S.currentJournal = null;
  S.src = null;
  S.plan = null;
  S.files = [];
  S.dests = null;
  S.prepError = null;
  S.phase = "waiting";
  resetRun();
}

let knownVolumes: Set<string> | null = null;

export async function pollVolumes() {
  try {
    const vols = await api.listSources();
    S.volumes = vols;
    const paths = new Set(vols.map((v) => v.path));
    const first = knownVolumes === null;
    const added = vols.filter((v) => !first && !knownVolumes!.has(v.path));
    knownVolumes = paths;

    // The card went away. Before an offload: back to waiting. After
    // one: keep showing the result, noting the card is gone.
    const gone = S.src && S.src.kind !== "folder" && ![...paths].some((p) => samePath(p, S.src!.path));
    if (gone && !S.scanning && !running()) {
      if (S.phase === "ready" && !S.resumeJournal) clearSource();
      else if ((S.phase === "done" || S.phase === "error") && !S.ejected) {
        S.ejected = true;
      }
    } else if (gone && S.released && !S.ejected) {
      S.ejected = true; // pulled during the backup, which reads from the copy
    }
    if (S.settings.autoSelect && !running() && !S.scanning) {
      // A newly inserted card or drive becomes the source; at launch, only an obvious card.
      const pick = first ? (S.src ? undefined : soleCard(vols)) : (added.find((v) => v.removable) ?? added[0]);
      if (pick) selectSource(pick.path, pick);
    }
  } catch {
    /* try again next tick */
  }
  setTimeout(pollVolumes, 2000);
}

function soleCard(vols: VolumeEntry[]) {
  const cards = vols.filter((v) => v.removable);
  return cards.length === 1 ? cards[0] : undefined;
}
