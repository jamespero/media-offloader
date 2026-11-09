// Maps engine events onto the panels and thumbnails.

import { failureText, type EngineEvent } from "./api";
import { toast } from "./components/Toast/toastState.svelte.ts";
import { finish } from "./offload";
import { resetSamples, sample } from "./runState";
import { ejectSource } from "./source";
import { S, totalBytes } from "./state.svelte.ts";

/** Plays the check-mark flash on a tile that just verified. */
function flashThumb(i: number, label: string) {
  const f = S.files[i];
  if (!f || S.view) return;
  f.flash = label;
  setTimeout(() => {
    if (f.flash === label) f.flash = undefined;
  }, 1000);
}

export function onEngine(e: EngineEvent) {
  switch (e.type) {
    case "passStarted":
      resetSamples();
      if (e.pass === "fanOut") S.runFanOut = true;
      S.rate = 0;
      S.eta = null;
      if (e.pass === "backup") {
        S.phase = "backing";
        S.cur = null;
      } else {
        S.phase = "copying";
      }
      break;
    case "fileStarted": {
      const f = S.files[e.file];
      if (!f) break;
      f.state = e.pass === "backup" ? "backing" : "copying";
      f.p = 0;
      f.verify = false;
      if (e.pass === "backup") S.curB = { name: f.name, verify: false };
      else S.cur = { name: f.name, verify: false };
      break;
    }
    case "progress": {
      const f = S.files[e.file];
      if (!f) break;
      f.verify = e.stage === "verify";
      f.p = f.verify ? 100 : f.size ? Math.min(100, (e.fileBytes / f.size) * 100) : 100;
      const pct = e.total ? (e.done / e.total) * 100 : 100;
      const live = { name: f.name, verify: f.verify };
      if (e.pass === "backup") {
        S.bakP = pct;
        S.curB = live;
      } else {
        S.copyP = pct;
        S.cur = live;
        if (e.pass === "fanOut") S.bakP = pct;
      }
      sample(e.done, e.total);
      break;
    }
    case "fileFinished": {
      const f = S.files[e.file];
      if (!f) break;
      const failed = e.results.filter(([, s]) => s.state === "failed");
      if (failed.length) {
        f.state = "failed";
        f.p = 0;
        const first = failed[0][1];
        if (first.state === "failed") f.failure = failureText(first.failure);
        for (const [dest] of failed) {
          if (dest === 0) S.failCopy++;
          else S.failBak++;
        }
      } else if (e.results.length && e.results.every(([, s]) => s.state === "verified")) {
        f.state = e.pass === "copy" || !S.runHadBackup ? "copied" : "backed";
        f.p = 100;
        f.failure = undefined;
        setTimeout(() => flashThumb(e.file, e.pass === "copy" ? "copy" : e.pass === "backup" ? "backup" : S.runHadBackup ? "copy · backup" : "copy"), 0);
      }
      // Catch the panel up: progress events are throttled and can skip a file's end.
      const doneBytes = S.files.slice(0, e.file + 1).reduce((a, x) => a + x.size, 0);
      const pct = totalBytes() ? (doneBytes / totalBytes()) * 100 : 100;
      if (e.pass === "backup") S.bakP = Math.max(S.bakP, pct);
      else {
        S.copyP = Math.max(S.copyP, pct);
        if (e.pass === "fanOut") S.bakP = S.copyP;
      }
      break;
    }
    case "sourceReleased":
      S.released = true;
      if (S.settings.ejectWhenVerified) ejectSource();
      break;
    case "warning":
      toast(false, "Heads up", e.message);
      break;
    case "passFinished":
      break;
    case "finished":
      finish(e.outcome);
      break;
  }
}
