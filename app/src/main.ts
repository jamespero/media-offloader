// Controller: source selection, destinations, starting/stopping, and mapping engine
// events onto the panels and thumbnails.

// In `vite dev` outside Tauri, fake the backend so the UI can be previewed in a browser.
// Must run before anything below talks to Tauri; compiled out of production builds.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  await import("./preview");
}

import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, failureText, type EngineEvent, type JobReport, type OffloadArgs, type Outcome, type VolumeEntry } from "./api";
import {
  about,
  addTokenMenu,
  chipMenu,
  closeMenus,
  closeOverlay,
  confirmStop,
  currentSettingsTab,
  destMenu,
  mainMenu,
  overlay,
  settings,
  showLog,
  srcMenu,
  toast,
} from "./menus";
import { $, byGridKind, esc, fmtSize, flashThumb, render, renderBak, renderCopy, renderFoot, renderSrc, renderStrip, updateLive, updateThumb } from "./render";
import { S, filesFromPlan, fileManager, hasBackup, running, saveSettings, sep, shownFiles, tokenContext, totalBytes, type Settings, type Source } from "./state";
import { resolveRoot, type Token, type TokenType } from "./tokens";
import { chooseAndReverify, initReverify, reverifyIntro, reverifyLog } from "./reverify";
import { markEarlierRun, openHistory, refreshHistory } from "./history";
import { chime, notify } from "./notify";
import { applyPreset, deletePreset, presetMenu, saveCurrentAs, savePresetDialog } from "./presets";
import { initThumbs, requestThumbs, stopThumbs } from "./thumbs";
import { historyApi, type HistoryDetail } from "./api";

/* ---------------------------------------------------------------- source */

let selectSeq = 0;

function resetRun() {
  S.ejected = false;
  S.copyP = S.bakP = 0;
  S.cur = S.curB = null;
  S.rate = 0;
  S.eta = null;
  S.released = false;
  S.failCopy = S.failBak = 0;
  S.abortReason = null;
  S.stoppedKept = 0;
  S.report = null;
  S.startedAt = S.finishedAt = null;
  samples = [];
}

function volumeMedia(v: VolumeEntry) {
  return `${v.fsType || "Drive"} · ${fmtSize(v.total)}`;
}

async function selectSource(path: string, hint?: VolumeEntry) {
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
  render();
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
  render();
  schedulePrepare();
  thumbsForCurrent();
}

/** Thumbnails for the Current tab: from the card before a run, from the verified copy after. */
function thumbsForCurrent() {
  if (!S.src || running()) return;
  const copy = S.dests?.[0]?.root;
  if ((S.phase === "done" || S.phase === "error") && copy) requestThumbs(byGridKind(S.files), copy, true);
  else if (S.phase === "ready") requestThumbs(byGridKind(S.files), S.src.path, false);
}

function thumbsForHistory() {
  const d = S.detail;
  if (!d) return;
  requestThumbs(byGridKind(S.histFiles), d.item.roots[0] ?? d.item.source, !!d.item.roots[0]);
}

function samePath(a: string, b: string) {
  const n = (p: string) => p.replace(/[\\/]+$/, "").toLowerCase();
  return n(a) === n(b);
}

/* ---------------------------------------------------------------- resume */

let resumeDetail: HistoryDetail | null = null;

/** Sets up the Current tab to continue an earlier offload (or retry its failed files). */
async function resumeFrom(path: string) {
  if (running()) return;
  closeOverlay();
  const seq = ++selectSeq;
  resumeDetail = null;
  try {
    resumeDetail = await historyApi.detail(path);
  } catch (e) {
    toast(false, "Couldn't read that offload", String(e));
    return;
  }
  if (seq !== selectSeq) return;
  const it = resumeDetail.item;
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
  render();
  await prepare();
  S.scanning = false;
  render();
  thumbsForCurrent();
}

/** Ejects the card (not while it's being read). */
async function ejectSource() {
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
  renderSrc();
}

function clearSource() {
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
  render();
}

/* ---------------------------------------------------------------- volumes */

let knownVolumes: Set<string> | null = null;

async function pollVolumes() {
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
        renderSrc();
      }
    } else if (gone && S.released && !S.ejected) {
      S.ejected = true; // pulled during the backup, which reads from the copy
      renderSrc();
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

/* ---------------------------------------------------------------- destinations */

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

function schedulePrepare() {
  clearTimeout(prepTimer);
  prepTimer = window.setTimeout(prepare, 250);
}

/** Plans against the real destinations: resolves copy-conflicted, checks space. */
async function prepare(): Promise<boolean> {
  clearTimeout(prepTimer);
  if (!S.src || (!S.settings.copyDest && !S.resumeJournal) || running() || S.phase !== "ready") {
    S.dests = null;
    S.prepError = null;
    if (S.phase === "ready") render();
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
    if (S.resumeJournal && resumeDetail) markEarlierRun(S.files, resumeDetail);
  } catch (e) {
    if (seq !== prepSeq) return false;
    S.dests = null;
    S.prepError = String(e);
  }
  render();
  return !S.prepError;
}

function setDest(w: "copy" | "bak", path: string) {
  const s = S.settings;
  if (w === "copy") s.copyDest = path;
  else s.bakDest = path;
  const key = w === "copy" ? "recentCopy" : "recentBak";
  s[key] = [path, ...s[key].filter((p) => p !== path)].slice(0, 5);
  saveSettings();
  S.dests = null;
  render();
  schedulePrepare();
}

function tokensChanged() {
  saveSettings();
  S.dests = null;
  renderCopy();
  renderBak();
  renderSrc();
  renderStrip();
  schedulePrepare();
}

/* ---------------------------------------------------------------- run */

let samples: [number, number][] = [];

async function startOffload() {
  if (!S.src || (!S.settings.copyDest && !S.resumeJournal) || running() || S.scanning) return;
  S.scanning = true;
  renderSrc();
  const ok = await prepare(); // fresh plan, with "now" tokens resolved at start time
  S.scanning = false;
  if (!ok) {
    render();
    return;
  }
  resetRun();
  S.runHadBackup = hasBackup();
  S.runFanOut = !S.resumeJournal && S.settings.fanOut; // a resumed run reports its own mode
  S.view = "";
  S.startedAt = new Date();
  S.phase = "copying";
  S.files.forEach((f) => Object.assign(f, { state: "pending", p: 0, verify: false, failure: undefined }));
  render();
  try {
    stopThumbs(); // the card is about to be read; thumbnails wait
    await api.start();
  } catch (e) {
    S.phase = "ready";
    toast(false, "Couldn't start", String(e));
    render();
  }
}

function sample(done: number, total: number) {
  const now = performance.now();
  samples.push([now, done]);
  while (samples.length > 2 && now - samples[0][0] > 4000) samples.shift();
  const [t0, d0] = samples[0];
  const dt = (now - t0) / 1000;
  if (dt > 0.6 && done > d0) {
    const work = (done - d0) / dt; // copy + verify bytes per second
    S.rate = work / 2; // the effective offload speed
    S.eta = (total - done) / work;
  }
}

function onEngine(e: EngineEvent) {
  switch (e.type) {
    case "passStarted":
      samples = [];
      if (e.pass === "fanOut") S.runFanOut = true;
      S.rate = 0;
      S.eta = null;
      if (e.pass === "backup") {
        S.phase = "backing";
        S.cur = null;
      } else {
        S.phase = "copying";
      }
      render();
      break;
    case "fileStarted": {
      const f = S.files[e.file];
      if (!f) break;
      f.state = e.pass === "backup" ? "backing" : "copying";
      f.p = 0;
      f.verify = false;
      if (e.pass === "backup") S.curB = { name: f.name, verify: false };
      else S.cur = { name: f.name, verify: false };
      updateThumb(e.file);
      updateLive();
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
      updateThumb(e.file);
      updateLive();
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
      updateThumb(e.file);
      updateLive();
      if (failed.length) renderSrc();
      break;
    }
    case "sourceReleased":
      S.released = true;
      renderSrc();
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

function finish(outcome: Outcome) {
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
  render();
  if (outcome.kind === "cancelled") schedulePrepare();
  thumbsForCurrent();
}

function onReport(r: JobReport) {
  S.report = r;
  S.failCopy = r.files.filter((f) => f.dests[0]?.state === "failed").length + (S.plan?.problems.length ?? 0);
  S.failBak = r.files.filter((f) => f.dests.slice(1).some((d) => d.state === "failed")).length;
  if (S.phase === "error" && S.failCopy + S.failBak === 0 && r.outcome.kind === "failed") S.failCopy = r.outcome.files;
  r.files.forEach((fr, i) => {
    const f = S.files[i];
    const bad = fr.dests.find((d) => d.state === "failed");
    if (f && bad && bad.state === "failed") f.failure = failureText(bad.failure);
  });
  render();
  refreshHistory().then(render);
  if (quitAfterStop) getCurrentWindow().destroy();
}

function newOffload() {
  if (running()) return;
  closeOverlay();
  clearSource();
}

/* ---------------------------------------------------------------- input */

document.addEventListener("click", async (ev) => {
  const target = ev.target as Element;
  const b = target.closest<HTMLElement>("[data-act]");
  if (!b) {
    if (!target.closest(".menu")) closeMenus();
    return;
  }
  if (b.hasAttribute("disabled")) return;
  const a = b.dataset.act!;
  ev.stopPropagation();
  if (a !== "tog" && !b.closest(".menu")) closeMenus();
  const s = S.settings;
  switch (a) {
    case "srcmenu":
      srcMenu(b);
      break;
    case "pick": {
      closeMenus();
      const v = S.volumes.find((x) => x.path === b.dataset.path);
      selectSource(b.dataset.path!, v);
      break;
    }
    case "pickfolder": {
      closeMenus();
      const p = await open({ directory: true, title: "Choose a folder to offload" });
      if (typeof p === "string") selectSource(p);
      break;
    }
    case "start":
      startOffload();
      break;
    case "dest":
      destMenu(b, b.dataset.w as "copy" | "bak");
      break;
    case "setdest":
      closeMenus();
      setDest(b.dataset.w as "copy" | "bak", b.dataset.v!);
      break;
    case "browsedest": {
      closeMenus();
      const w = b.dataset.w as "copy" | "bak";
      const current = (w === "copy" ? s.copyDest : s.bakDest) ?? undefined;
      const p = await open({ directory: true, title: w === "copy" ? "Copy media to…" : "Back up media to…", defaultPath: current });
      if (typeof p === "string") setDest(w, p);
      break;
    }
    case "rmbak":
      s.bakDest = null;
      saveSettings();
      S.dests = null;
      render();
      schedulePrepare();
      break;
    case "addtoken":
      addTokenMenu(b);
      break;
    case "addtok": {
      closeMenus();
      const t = b.dataset.t as TokenType;
      const tk: Token = t === "now" || t === "shot" ? { t, fmt: "YY-MM-DD" } : t === "day" ? { t, fmt: "Day01" } : t === "text" ? { t, v: "text" } : t === "reel" ? { t, v: "A" } : t === "project" ? { t, v: "Project" } : { t };
      s.tokens.push(tk);
      tokensChanged();
      break;
    }
    case "chip":
      chipMenu(b, +b.dataset.i!, tokensChanged);
      break;
    case "fmt":
      closeMenus();
      s.tokens[+b.dataset.i!].fmt = b.dataset.f;
      tokensChanged();
      break;
    case "reel":
      closeMenus();
      s.tokens[+b.dataset.i!].v = b.dataset.v;
      tokensChanged();
      break;
    case "rmtok":
      closeMenus();
      s.tokens.splice(+b.dataset.i!, 1);
      tokensChanged();
      break;
    case "mvl":
    case "mvr": {
      closeMenus();
      const i = +b.dataset.i!;
      const j = a === "mvl" ? i - 1 : i + 1;
      [s.tokens[i], s.tokens[j]] = [s.tokens[j], s.tokens[i]];
      tokensChanged();
      break;
    }
    case "cancel":
      confirmStop();
      break;
    case "dostop":
      closeOverlay();
      api.cancel();
      break;
    case "log":
      closeMenus();
      if (S.report?.logPath) showLog(S.report.logPath);
      break;
    case "logsfolder": {
      closeMenus();
      const d = await api.logDir();
      if (d) api.openFolder(d).catch((e) => toast(false, "Couldn't open the logs folder", String(e)));
      break;
    }
    case "revealcopy":
      closeMenus();
      if (S.dests?.[0]) api.reveal(S.dests[0].root).catch(() => {});
      break;
    case "revealpath":
      api.reveal(b.dataset.path!).catch(() => {});
      break;
    case "settings":
      closeMenus();
      settings();
      break;
    case "stab":
      settings(b.dataset.tab);
      break;
    case "tog": {
      const k = b.dataset.k as keyof Settings;
      if (typeof s[k] === "boolean") (s as unknown as Record<string, boolean>)[k] = !s[k];
      saveSettings();
      settings(currentSettingsTab());
      renderFoot();
      if (k === "includeSystemFiles" && S.src && !running()) selectSource(S.src.path);
      break;
    }
    case "md5":
      s.md5 = b.dataset.v === "1";
      saveSettings();
      settings("verify");
      renderFoot();
      break;
    case "fan":
      s.fanOut = b.dataset.v === "1";
      saveSettings();
      settings("transfers");
      renderFoot();
      renderBak();
      break;
    case "closeov":
      closeOverlay();
      break;
    case "new":
      closeMenus();
      newOffload();
      break;
    case "tab": {
      const path = b.dataset.path ?? "";
      if (!path) {
        S.view = "";
        S.kind = null;
        S.detail = null;
        render();
        thumbsForCurrent();
      } else {
        S.view = path;
        render(); // shows "Loading…" until the journal is read
        await openHistory(path);
        render();
        const d = S.detail;
        if (d && S.view === path) thumbsForHistory();
      }
      break;
    }
    case "forget": {
      const path = b.dataset.path!;
      try {
        await historyApi.forget(path);
      } catch (e) {
        toast(false, "Couldn't remove it from the history", String(e));
      }
      if (S.view === path) S.view = "";
      await refreshHistory();
      render();
      break;
    }
    case "resume":
      resumeFrom(b.dataset.path!);
      break;
    case "histlog":
      if (S.detail?.item.log) showLog(S.detail.item.log);
      break;
    case "eject":
      closeMenus();
      ejectSource();
      break;
    case "kind":
      S.kind = b.dataset.k as typeof S.kind;
      render();
      // Thumbnails for what's now shown come first.
      if (S.view) thumbsForHistory();
      else thumbsForCurrent();
      break;
    case "presets":
      presetMenu(b);
      break;
    case "usepreset":
      closeMenus();
      applyPreset(b.dataset.id!);
      S.dests = null;
      render();
      schedulePrepare();
      break;
    case "savepreset":
      closeMenus();
      savePresetDialog();
      break;
    case "dosavepreset": {
      const name = (document.getElementById("presetName") as HTMLInputElement | null)?.value ?? "";
      if (saveCurrentAs(name)) {
        renderStrip();
        toast(true, "Preset saved", name.trim());
      }
      break;
    }
    case "managepresets":
      closeMenus();
      settings("naming");
      break;
    case "delpreset":
      deletePreset(b.dataset.id!);
      settings("naming");
      renderStrip();
      break;
    case "importinc":
    case "importdec":
      s.importNo = Math.max(1, s.importNo + (a === "importinc" ? 1 : -1));
      saveSettings();
      settings("naming");
      tokensChanged();
      break;
    case "about":
      closeMenus();
      about();
      break;
    case "reverify":
      closeMenus();
      reverifyIntro();
      break;
    case "rvchoose":
      closeOverlay();
      chooseAndReverify();
      break;
    case "rvlog":
      reverifyLog();
      break;
  }
});

$("menuBtn").addEventListener("click", (e) => {
  e.stopPropagation();
  mainMenu($("menuBtn"));
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    closeMenus();
    closeOverlay();
    return;
  }
  const cmd = e.ctrlKey || e.metaKey;
  if (!cmd || (e.target as Element).closest("input")) return;
  if (e.key === "n") {
    e.preventDefault();
    newOffload();
  } else if (e.key === "l" && S.report?.logPath) {
    e.preventDefault();
    showLog(S.report.logPath);
  } else if (e.key === "r") {
    e.preventDefault();
    reverifyIntro();
  } else if (e.key === ",") {
    e.preventDefault();
    settings();
  }
});
window.addEventListener("resize", closeMenus);

/* Double-click a thumbnail: show the verified copy (or the original) in the file manager. */
document.addEventListener("dblclick", (e) => {
  const t = (e.target as Element).closest<HTMLElement>(".thumb");
  if (!t) return;
  const f = shownFiles()[+t.dataset.i!];
  if (!f) return;
  const native = (rel: string) => rel.split("/").join(sep);
  const copied = f.state === "copied" || f.state === "backed" || f.state === "backing";
  const copyRoot = S.view ? S.detail?.item.roots[0] : S.dests?.[0]?.root;
  const sourceRoot = S.view ? S.detail?.item.source : S.src?.path;
  const path =
    copied && copyRoot
      ? copyRoot + sep + native(f.destRel)
      : sourceRoot
        ? sourceRoot.replace(/[\\/]+$/, "") + sep + native(f.srcRel)
        : null;
  if (path) api.reveal(path).catch(() => {});
});

/* Tooltip on thumbnails. */
const tip = $("tip");
document.addEventListener("mousemove", (e) => {
  const t = (e.target as Element).closest<HTMLElement>(".thumb");
  const f = t ? shownFiles()[+t.dataset.i!] : undefined;
  if (!f) {
    tip.style.display = "none";
    return;
  }
  const st = {
    pending: "Not copied yet",
    copying: f.verify ? "Verifying…" : "Copying…",
    copied: "Copied & verified",
    backing: f.verify ? "Verifying backup…" : "Backing up…",
    backed: "Copied & backed up · verified",
    failed: f.failure ?? "Not verified",
  }[f.state];
  const color = f.state === "failed" ? "var(--err)" : /backed|copied/.test(f.state) ? "var(--done)" : "var(--tx2)";
  const when = f.modified ? new Date(f.modified) : null;
  tip.innerHTML = `<b>${esc(f.name)}</b>${fmtSize(f.size)}${when ? ` · ${when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} ${when.toTimeString().slice(0, 8)}` : ""}
    <div class="h">${esc(f.srcRel)}</div><div style="margin-top:5px;color:${color}">${esc(st)}</div>
    <div class="h">Double-click to show in ${fileManager}</div>`;
  tip.style.display = "block";
  const x = Math.min(e.clientX + 14, innerWidth - tip.offsetWidth - 8);
  const y = Math.min(e.clientY + 16, innerHeight - tip.offsetHeight - 8);
  tip.style.left = x + "px";
  tip.style.top = y + "px";
});

/* Closing the window mid-offload asks first, then stops cleanly. */
let quitAfterStop = false;
getCurrentWindow().onCloseRequested((event) => {
  if (!running()) return;
  event.preventDefault();
  overlay(`<div class="dialog"><h3>Stop and quit?</h3>
    <p>An offload is still running. Verified files are kept; the file in progress is removed.</p>
    <div class="acts"><button class="btn" data-act="closeov">Keep going</button><button class="btn danger" id="quitBtn">Stop and quit</button></div></div>`);
  document.getElementById("quitBtn")!.onclick = () => {
    quitAfterStop = true;
    closeOverlay();
    api.cancel();
    setTimeout(() => getCurrentWindow().destroy(), 5000); // in case the report never comes
  };
});

/* ---------------------------------------------------------------- boot */

api.onEngine(onEngine);
api.onReport(onReport);
initThumbs();
initReverify();
render();
refreshHistory().then(render);
pollVolumes();

// Dev builds only: a handle for driving the app from automated end-to-end checks.
if (import.meta.env.DEV) Object.assign(window, { __offload: { S, selectSource, setDest, startOffload, clearSource } });
