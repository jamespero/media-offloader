// What the copy and backup panels should show, derived from the app state.

import { fmtDuration } from "./format";
import { S, hasBackup, sep, tokenContext } from "./state.svelte.ts";
import { resolveRoot } from "./tokens";

export type PanelState = "idle" | "queued" | "running" | "done" | "error" | "hold";

/** The history item being resumed on the Current tab, if any. */
export const resuming = () => (S.resumeJournal ? S.history.find((h) => h.path === S.resumeJournal) : undefined);

/** Where a destination's files will go: the prepared root if we have one, else computed. */
export function rootFor(which: "copy" | "bak"): string | null {
  const base = which === "copy" ? S.settings.copyDest : S.settings.bakDest;
  if (!base) return null;
  const prepared = S.dests?.find((d) => d.role === (which === "copy" ? "copy" : "backup"));
  return prepared?.root ?? resolveRoot(base, S.settings.tokens, tokenContext(), sep);
}

export function copyState(): PanelState {
  switch (S.phase) {
    case "waiting":
    case "ready":
      return "idle";
    case "copying":
      return "running";
    case "backing":
      return S.runFanOut ? "running" : "done";
    case "done":
      return "done";
    case "error":
      return S.failCopy > 0 || (S.abortReason && !S.released) ? "error" : "done";
  }
}

export function bakState(): PanelState {
  if (!hasBackup() || (!S.runHadBackup && (S.phase === "done" || S.phase === "error"))) return "idle";
  switch (S.phase) {
    case "waiting":
    case "ready":
      return "idle";
    case "copying":
      return S.runFanOut ? "running" : "queued";
    case "backing":
      return "running";
    case "done":
      return "done";
    case "error":
      if (copyState() === "error") return S.runFanOut && S.failBak > 0 ? "error" : "hold";
      return S.failBak > 0 || S.abortReason ? "error" : "done";
  }
}

export function rateText() {
  if (!S.rate) return "";
  return `${(S.rate / 1e9).toFixed(2)} GB/s${S.eta != null ? ` · ${fmtDuration(S.eta)} left` : ""}`;
}
