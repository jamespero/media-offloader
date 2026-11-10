// Running an offload: starting it, recording how it ended, and stopping cleanly when the
// window closes mid-run.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, failureText, type JobReport, type Outcome } from "./api";
import QuitDialog from "./components/Dialogs/QuitDialog.svelte";
import { closeOverlay, overlay } from "./components/Overlay/overlayState.svelte.ts";
import { toast } from "./components/Toast/toastState.svelte.ts";
import { fmtSize } from "./format";
import { prepare, schedulePrepare } from "./destinations";
import { refreshHistory } from "./history";
import { chime, notify } from "./notify";
import { resetRun } from "./runState";
import { clearSource } from "./source";
import { S, hasBackup, running, saveSettings, totalBytes } from "./state.svelte.ts";
import { thumbsForCurrent } from "./thumbRequests";
import { stopThumbs } from "./thumbs";

let quitAfterStop = false;

export async function startOffload() {
  if (!S.src || (!S.settings.copyDest && !S.resumeJournal) || running() || S.scanning) return;
  S.scanning = true;
  const ok = await prepare(); // fresh plan, with "now" tokens resolved at start time
  S.scanning = false;
  if (!ok) {
    return;
  }
  resetRun();
  S.runHadBackup = hasBackup();
  S.runFanOut = !S.resumeJournal && S.settings.fanOut; // a resumed run reports its own mode
  S.view = "";
  S.startedAt = new Date();
  S.phase = "copying";
  S.files.forEach((f) => Object.assign(f, { state: "pending", p: 0, verify: false, failure: undefined }));
  try {
    stopThumbs(); // the card is about to be read; thumbnails wait
    await api.start();
  } catch (e) {
    S.phase = "ready";
    toast(false, "Couldn't start", String(e));
  }
}

export function finish(outcome: Outcome) {
  S.finishedAt = new Date();
  S.cur = S.curB = null;
  S.rate = 0;
  S.eta = null;
  const drives = S.runHadBackup ? "2 drives" : "1 drive";
  switch (outcome.kind) {
    case "verified": {
      S.phase = "done";
      S.copyP = S.bakP = 100;
      S.files.forEach((f) => {
        f.state = S.runHadBackup ? "backed" : "copied";
        f.p = 100;
      });
      // Counters move on after every completed offload.
      const s = S.settings;
      s.importNo += 1;
      for (const t of s.tokens) if (t.t === "reel") s.reels[t.v || "A"] = (s.reels[t.v || "A"] ?? 1) + 1;
      saveSettings();
      toast(true, "Offload complete", `${fmtSize(totalBytes())} verified on ${drives}`);
      chime(true);
      notify("Offload complete", `${S.src?.label ?? "Media"}: ${fmtSize(totalBytes())} verified on ${drives}.`);
      break;
    }
    case "failed":
      S.phase = "error";
      if (S.failCopy + S.failBak === 0) S.failCopy = outcome.files;
      toast(false, "Offload not complete", `${outcome.files} file${outcome.files === 1 ? "" : "s"} didn't verify. Keep the card.`);
      chime(false);
      notify("Offload not complete", `${outcome.files} file${outcome.files === 1 ? "" : "s"} didn't verify. Keep the card.`);
      break;
    case "cancelled":
      S.phase = "ready";
      S.stoppedKept = S.files.filter((f) => f.state === "copied" || f.state === "backed").length;
      break;
    case "aborted":
      S.phase = "error";
      S.abortReason = outcome.reason;
      toast(false, "Offload stopped", outcome.reason);
      chime(false);
      notify("Offload stopped", outcome.reason);
      break;
  }
  if (outcome.kind === "cancelled") schedulePrepare();
  thumbsForCurrent();
}

export function onReport(r: JobReport) {
  S.report = r;
  S.failCopy = r.files.filter((f) => f.dests[0]?.state === "failed").length + (S.plan?.problems.length ?? 0);
  S.failBak = r.files.filter((f) => f.dests.slice(1).some((d) => d.state === "failed")).length;
  if (S.phase === "error" && S.failCopy + S.failBak === 0 && r.outcome.kind === "failed") S.failCopy = r.outcome.files;
  r.files.forEach((fr, i) => {
    const f = S.files[i];
    const bad = fr.dests.find((d) => d.state === "failed");
    if (f && bad && bad.state === "failed") f.failure = failureText(bad.failure);
  });
  void refreshHistory();
  if (quitAfterStop) getCurrentWindow().destroy();
}

/** Stops the running offload, once the user has confirmed. */
export function stopOffload() {
  closeOverlay();
  api.cancel();
}

export function newOffload() {
  if (running()) return;
  closeOverlay();
  clearSource();
}

/** Closing the window mid-offload asks first, then stops cleanly. */
export function guardWindowClose() {
  getCurrentWindow().onCloseRequested((event) => {
    if (!running()) return;
    event.preventDefault();
    overlay(QuitDialog, {
      onQuit: () => {
        quitAfterStop = true;
        closeOverlay();
        api.cancel();
        setTimeout(() => getCurrentWindow().destroy(), 5000); // in case the report never comes
      },
    });
  });
}
