// History: past offloads read from their journals, and Resume / Retry for those that
// didn't finish. The views are in components/History and components/Strip.

import { historyApi, type HistoryDetail, type HistoryItem } from "./api";
import { toast } from "./components/Toast/toastState.svelte.ts";
import { S, fileView, type FileView } from "./state.svelte.ts";
import { thumbsForCurrent, thumbsForHistory } from "./thumbRequests";

export async function refreshHistory() {
  try {
    S.history = await historyApi.list(30);
  } catch {
    S.history = [];
  }
}

const p2 = (n: number) => String(n).padStart(2, "0");
export const clock = (ms: number) => {
  const d = new Date(ms);
  return `${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`;
};
export const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });

export function itemState(it: HistoryItem): "ok" | "err" | "" {
  if (!it.outcome) return "err"; // didn't finish
  if (it.outcome.kind === "verified") return "ok";
  if (it.outcome.kind === "cancelled") return "";
  return "err";
}

export async function openHistory(path: string) {
  S.view = path;
  S.kind = null;
  S.detail = null;
  S.histFiles = [];
  try {
    S.detail = await historyApi.detail(path);
  } catch {
    S.view = "";
    return;
  }
  if (S.view !== path) return;
  S.histFiles = filesOf(S.detail);
}

/** Shows a past offload, or the current one when `path` is empty. */
export async function showTab(path: string) {
  if (!path) {
    S.view = "";
    S.kind = null;
    S.detail = null;
    thumbsForCurrent();
    return;
  }
  S.view = path;
  await openHistory(path);
  if (S.detail && S.view === path) thumbsForHistory();
}

/** Removes an offload from the history (its journal), leaving its files alone. */
export async function forgetOffload(path: string) {
  try {
    await historyApi.forget(path);
  } catch (e) {
    toast(false, "Couldn't remove it from the history", String(e));
  }
  if (S.view === path) S.view = "";
  await refreshHistory();
}

/** Shows one kind of file in the grid, asking for its thumbnails first. */
export function showKind(kind: typeof S.kind) {
  S.kind = kind;
  if (S.view) thumbsForHistory();
  else thumbsForCurrent();
}

function filesOf(d: HistoryDetail): FileView[] {
  const sets = d.verified.map((v) => new Set(v));
  const many = d.item.roots.length > 1;
  return d.files.map((f) => {
    const v = fileView(f.dest, f.src, f.size, f.modified);
    const failure = d.failed.map((m) => m[f.dest]).find(Boolean);
    if (failure) {
      v.state = "failed";
      v.failure = failure;
    } else if (sets.length && sets.every((s) => s.has(f.dest))) {
      v.state = many ? "backed" : "copied";
      v.p = 100;
    } else if (sets[0]?.has(f.dest)) {
      v.state = "copied";
      v.p = 100;
    }
    return v;
  });
}

/** Marks files an earlier run already verified, for the resume preview. */
export function markEarlierRun(files: FileView[], d: HistoryDetail) {
  const sets = d.verified.map((v) => new Set(v));
  const many = d.item.roots.length > 1;
  for (const f of files) {
    if (sets.length && sets.every((s) => s.has(f.destRel))) {
      f.state = many ? "backed" : "copied";
      f.p = 100;
    } else if (sets[0]?.has(f.destRel)) {
      f.state = "copied";
      f.p = 100;
    }
  }
}

export function resumeLabel(it: HistoryItem) {
  const failed = Math.max(0, ...it.failed);
  return it.outcome?.kind === "failed" && failed > 0 ? `Retry ${failed} file${failed === 1 ? "" : "s"}` : "Resume";
}
