// Controller: startup and input wiring.

// In `vite dev` outside Tauri, fake the backend so the UI can be previewed in a browser.
// Must run before anything below talks to Tauri; compiled out of production builds.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && "invoke" in (window as any).__TAURI_INTERNALS__)) {
  await import("./preview");
}

import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
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
  settings,
  showLog,
  srcMenu,
  toast,
} from "./menus";
import { $, esc, fmtSize, render, renderBak, renderFoot, renderStrip } from "./render";
import { S, fileManager, running, saveSettings, sep, shownFiles, type Settings } from "./state";
import type { Token, TokenType } from "./tokens";
import { chooseAndReverify, initReverify, reverifyIntro, reverifyLog } from "./reverify";
import { openHistory, refreshHistory } from "./history";
import { applyPreset, deletePreset, presetMenu, saveCurrentAs, savePresetDialog } from "./presets";
import { initThumbs } from "./thumbs";
import { initUpdates } from "./updates";
import { mountFooter } from "./components/Footer/footerState.svelte.ts";
import { historyApi } from "./api";
import { schedulePrepare, setDest, tokensChanged } from "./destinations";
import { onEngine } from "./engineEvents";
import { guardWindowClose, newOffload, onReport, startOffload } from "./offload";
import { clearSource, ejectSource, pollVolumes, resumeFrom, selectSource } from "./source";
import { thumbsForCurrent, thumbsForHistory } from "./thumbRequests";

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
    case "openrepo":
      api.openRepo().catch(() => {});
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
      void about();
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
    tip.classList.remove("on");
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
  const stateClass = f.state === "failed" ? " problem" : /backed|copied/.test(f.state) ? " done" : "";
  const when = f.modified ? new Date(f.modified) : null;
  tip.innerHTML = `<b>${esc(f.name)}</b>${fmtSize(f.size)}${when ? ` · ${when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} ${when.toTimeString().slice(0, 8)}` : ""}
    <div class="h">${esc(f.srcRel)}</div><div class="state${stateClass}">${esc(st)}</div>
    <div class="h">Double-click to show in ${fileManager}</div>`;
  tip.classList.add("on");
  const x = Math.min(e.clientX + 14, innerWidth - tip.offsetWidth - 8);
  const y = Math.min(e.clientY + 16, innerHeight - tip.offsetHeight - 8);
  tip.style.setProperty("--x", x + "px");
  tip.style.setProperty("--y", y + "px");
});

guardWindowClose();

/* ---------------------------------------------------------------- boot */

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
