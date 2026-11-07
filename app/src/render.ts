// Rendering. Full re-renders on state changes; cheap in-place updates for progress.

import { historyTabs, interruptedBanner, renderHistory, resumeLabel } from "./history";
import { presetButton } from "./presets";
import { thumbUrl } from "./thumbs";
import { ic } from "./icons";
import { S, sep, totalBytes, running, hasBackup, shownFiles, type FileView, tokenContext } from "./state";
import { resolveRoot, tokenInfo, hasFormat } from "./tokens";

/** The history item being resumed on the Current tab, if any. */
const resuming = () => (S.resumeJournal ? S.history.find((h) => h.path === S.resumeJournal) : undefined);

export const $ = (id: string) => document.getElementById(id)!;
export const esc = (s: unknown) =>
  String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

export function fmtSize(b: number): string {
  if (b >= 1e12) return (b / 1e12).toFixed(2) + " TB";
  if (b >= 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b >= 1e6) return Math.round(b / 1e6) + " MB";
  if (b >= 1e3) return Math.round(b / 1e3) + " KB";
  return b + " B";
}

export function fmtDuration(s: number): string {
  s = Math.max(1, Math.round(s));
  if (s >= 3600) return `${Math.floor(s / 3600)}h ${String(Math.floor((s % 3600) / 60)).padStart(2, "0")}m`;
  if (s >= 60) return `${Math.floor(s / 60)}m ${String(s % 60).padStart(2, "0")}s`;
  return `${s}s`;
}

/** Where a destination's files will go: the prepared root if we have one, else computed. */
export function rootFor(which: "copy" | "bak"): string | null {
  const base = which === "copy" ? S.settings.copyDest : S.settings.bakDest;
  if (!base) return null;
  const prepared = S.dests?.find((d) => d.role === (which === "copy" ? "copy" : "backup"));
  return prepared?.root ?? resolveRoot(base, S.settings.tokens, tokenContext(), sep);
}

function pathLine(p: string, cls = "path") {
  // rtl + bdi keeps the end of a long path (the interesting part) visible.
  return `<div class="${cls}" title="${esc(p)}"><bdi>${esc(p)}</bdi></div>`;
}

/* ---------------------------------------------------------------- panel status */

export type PanelState = "idle" | "queued" | "running" | "done" | "error" | "hold";

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

/* ---------------------------------------------------------------- source */

function srcIcon() {
  if (!S.src) return `<div class="waiting-icon">${S.scanning ? `<span class="spin">${ic.spinner(36)}</span>` : ic.cardWaiting()}</div>`;
  const k = S.src.kind;
  return `<div class="srcicon">${k === "card" ? ic.card() : k === "drive" ? ic.drive() : ic.folder()}</div>`;
}

/** Whether the card is being read right now (so it mustn't be ejected). */
const cardBusy = () => S.phase === "copying" || (S.phase === "backing" && S.runFanOut);
const canEject = () => !!S.src && S.src.kind !== "folder" && !S.ejected && !cardBusy() && !S.scanning;

function volBtn() {
  const s = S.src!;
  const pick = running() ? "" : `data-act="srcmenu"`;
  const ej = canEject() ? `<button class="iconbtn" style="width:24px;height:24px" title="Eject" data-act="eject">${ic.eject(14)}</button>` : "";
  return `<div class="row" style="gap:2px"><button class="vol" ${pick} title="${esc(s.path)}"><span>${esc(s.label)}</span> ${running() ? "" : ic.chev()}</button>${ej}</div>
    <div class="meta">${esc(s.media)}</div>`;
}

/** After the copy verified: an Eject button, or confirmation that it was ejected. */
function ejectRow(rest: string) {
  if (S.ejected) return `<div class="sub">Ejected — safe to remove. ${rest}</div>`;
  const btn = canEject() ? `<button class="btn sm" data-act="eject">${ic.eject(13)} Eject</button>` : "";
  return `<div class="sub">Safe to remove. ${rest}</div>${btn}`;
}

export function renderSrc() {
  if (S.view) return; // a history tab is showing
  const el = $("pSrc");
  let h = "";
  const ph = S.phase;
  if (!S.src) {
    h = `${srcIcon()}<h2>source</h2><div class="sub">${S.scanning ? "Reading the source…" : "Waiting for media"}</div>
      <button class="btn sm" data-act="srcmenu">Select a source ${ic.chev()}</button>`;
  } else if (ph === "ready" || ph === "waiting") {
    const again = S.dests?.[0]?.conflict != null;
    const res = resuming();
    const can = (!!S.settings.copyDest || !!S.resumeJournal) && !S.scanning && !S.prepError;
    const why = S.prepError ? "Fix the destination first." : !S.settings.copyDest && !res ? "Choose where to copy first." : "";
    const label = res ? resumeLabel(res) : again ? "Offload again" : "Start";
    const sub = S.scanning
      ? "Reading the source…"
      : res
        ? "Picks up where it stopped. Files already verified aren't copied again."
        : "Media files have not been copied from this source.";
    h = `${srcIcon()}<h2>source</h2>${volBtn()}<div class="sub">${sub}</div>
      <button class="btn primary" data-act="start" style="margin-top:4px" ${can ? "" : "disabled"} title="${esc(why)}">${label}</button>`;
  } else if (ph === "copying" || (ph === "backing" && S.runFanOut)) {
    h = `${srcIcon()}<h2>source</h2>${volBtn()}
      <div class="row" style="margin-top:4px"><span class="pulse"></span><span class="sub" style="color:var(--tx)">Copying from this source</span></div>
      <div class="sub">Keep it connected until the copy is verified.</div>`;
  } else if (ph === "backing") {
    h = `${srcIcon()}<h2>source</h2>${volBtn()}<span class="safe">${ic.check(13)} Copy verified</span>
      ${ejectRow("The backup continues from the copy.")}`;
  } else if (ph === "done") {
    h = `${srcIcon()}<h2>source</h2>${volBtn()}<span class="safe">${ic.check(13)} Copied &amp; verified</span>
      ${S.src.kind === "folder" ? `<div class="sub">Media files have been copied and verified.</div>` : ejectRow("Media files have been copied and verified.")}`;
  } else if (ph === "error") {
    if (S.released) {
      h = `${srcIcon()}<h2>source</h2>${volBtn()}<span class="safe">${ic.check(13)} Copy verified</span>
        ${ejectRow("The backup didn't finish.")}`;
    } else {
      const label = S.failCopy > 0 ? `${S.failCopy} file${S.failCopy === 1 ? "" : "s"} failed` : "Stopped";
      h = `${srcIcon()}<h2>source</h2>${volBtn()}<span class="bad">${ic.alert(13)} ${label}</span>
        <div class="sub">Keep this card. ${S.failCopy > 0 ? "Some files didn't verify." : "The copy isn't complete."}</div>`;
    }
  }
  el.innerHTML = `<div class="content">${h}</div>`;
}

/* ---------------------------------------------------------------- tokens */

function tokensRow() {
  const chips = S.settings.tokens
    .map((t, i) => {
      const info = tokenInfo(t.t);
      const label =
        t.t === "now" ? `<span class="k">Now</span> ${esc(t.fmt)}`
        : t.t === "shot" ? `<span class="k">Shot</span> ${esc(t.fmt)}`
        : t.t === "text" ? esc(t.v || "text")
        : t.t === "day" ? `<span class="k">Day</span> ${esc(t.fmt ?? "Day01")}`
        : t.t === "reel" ? `<span class="k">Reel</span> ${esc(t.v || "A")}###`
        : t.t === "project" ? `<span class="k">Project</span> ${esc(t.v || "Project")}`
        : info.label;
      const icon = t.t !== "text" ? ic[info.icon]() : "";
      const chev = hasFormat(t.t) || t.t === "reel" ? ic.chev(10) : "";
      return `<button class="chip ${t.t === "text" ? "text" : ""}" data-act="chip" data-i="${i}">${icon}${label}${chev}</button>`;
    })
    .join("");
  return `<div class="lbl">Add subfolders</div><div class="chips">${chips}<button class="addchip" data-act="addtoken" title="Add subfolder">${ic.plus()}</button></div>`;
}

function previewLine(which: "copy" | "bak") {
  if (S.prepError && which === "copy") return `<div class="preview"><span class="warn">${esc(S.prepError)}</span></div>`;
  const root = rootFor(which);
  if (!root || !S.src) return "";
  return `<div class="preview" title="${esc(root)}">→ <b>${esc(root)}${esc(sep)}</b></div>`;
}

/* ---------------------------------------------------------------- copy / backup */

function liveBlock(heading: string, root: string, pct: number, cur: { name: string; verify: boolean } | null, rate: string) {
  return `<h2>${heading}</h2>${pathLine(root)}<div class="pct"><span class="n">${Math.floor(pct)}%</span><small>complete</small></div>
    <div class="status">${statusHtml(cur)}</div><div class="rate">${esc(rate)}</div>`;
}

function statusHtml(cur: { name: string; verify: boolean } | null) {
  return cur ? `<span>${cur.verify ? "Verifying checksum" : "Copying"}</span><span class="f">${esc(cur.name)}</span>` : "";
}

export function rateText() {
  if (!S.rate) return "";
  return `${(S.rate / 1e9).toFixed(2)} GB/s${S.eta != null ? ` · ${fmtDuration(S.eta)} left` : ""}`;
}

function summaryPill(verb: string) {
  return `<button class="summary" data-act="log" title="Open the log for this offload">${fmtSize(totalBytes())} in ${S.files.length} file${S.files.length === 1 ? "" : "s"} · ${verb} &amp; checksum verified ${ic.info(12)}</button>`;
}

/** A destination panel before resuming: where it continues and how much is done. */
function resumePanel(name: "copy" | "backup", d: number) {
  const dest = S.dests?.[d];
  const it = resuming();
  const done = it?.verified[d] ?? 0;
  const of = it?.files ?? S.files.length;
  return `<h2>${name}</h2><div class="sub">Continuing in the same folder:</div>
    ${dest ? pathLine(dest.root) : ""}
    <div class="preview"><b>${done} of ${of}</b> files already verified</div>`;
}

export function renderCopy() {
  if (S.view) return; // a history tab is showing
  const el = $("pCopy");
  const st = copyState();
  let h = "", extra = "";
  el.className = "panel";
  const root = rootFor("copy") ?? "";
  if (st === "idle" && S.resumeJournal) {
    h = resumePanel("copy", 0);
  } else if (st === "idle") {
    el.classList.toggle("dim", !S.src);
    const dest = S.settings.copyDest;
    h = `<h2>copy</h2><div class="sub">Media files will be copied to:</div>
      <button class="btn sm" data-act="dest" data-w="copy">${ic.folder(14)} Choose…</button>
      ${dest ? pathLine(dest + (dest.endsWith(sep) ? "" : sep)) : `<div class="path">No folder chosen</div>`}
      ${tokensRow()}${previewLine("copy")}`;
  } else if (st === "running") {
    el.classList.add("running");
    extra = `<div class="fill" style="width:${S.copyP}%"></div><span class="tagline">${S.runFanOut ? "Fan-out" : "Copy"}</span>
      <button class="corner" data-act="cancel" title="Stop">${ic.x()}</button>`;
    h = liveBlock("copying to", root, S.copyP, S.cur, rateText());
  } else if (st === "error") {
    el.classList.add("error");
    const n = S.failCopy;
    const big = n > 0 ? `${n} file${n === 1 ? "" : "s"}<small>failed verification</small>` : `stopped<small>${esc(S.abortReason ?? "")}</small>`;
    h = `<h2>copy incomplete</h2>${pathLine(root)}<div class="pct" style="font-size:44px">${big}</div>
      <div class="row" style="margin-top:8px"><button class="btn sm" data-act="log">${ic.file(13)} Show log</button></div>`;
  } else {
    el.classList.add("done");
    h = `<h2>copied to</h2>${pathLine(root)}<div class="pct">100%<small>complete</small></div>
      <div class="okring">${ic.check(20, 2.6)}</div>${summaryPill("copied")}`;
  }
  el.innerHTML = extra + `<div class="content">${h}</div>`;
}

export function renderBak() {
  if (S.view) return; // a history tab is showing
  const el = $("pBak");
  const st = bakState();
  let h = "", extra = "";
  el.className = "panel";
  const root = rootFor("bak") ?? "";
  if (st === "idle" && S.resumeJournal) {
    el.classList.toggle("dim", !hasBackup());
    h = hasBackup() ? resumePanel("backup", 1) : `<h2>backup</h2><div class="sub">No backup in this offload.</div>`;
  } else if (!hasBackup() || (st === "idle" && running())) {
    el.classList.toggle("dim", !S.src || running());
    h = `<h2>backup</h2><div class="sub">Select a location to save a second copy of all media files.</div>
      <button class="btn sm" data-act="dest" data-w="bak">${ic.folder(14)} Choose…</button>`;
  } else if (st === "idle") {
    el.classList.toggle("dim", !S.src);
    if (S.src) extra = `<button class="corner" data-act="rmbak" title="Remove backup destination">${ic.x(14)}</button>`;
    const dest = S.settings.bakDest ?? "";
    h = `<h2>backup</h2><div class="sub">Media files will also be copied to:</div>
      <button class="btn sm" data-act="dest" data-w="bak">${ic.folder(14)} Choose…</button>
      ${pathLine(dest + (dest.endsWith(sep) ? "" : sep))}
      <div class="lbl">Subfolders</div><div class="chips"><span class="chip" style="cursor:default">${ic.layers(13)} Same as copy</span></div>
      ${previewLine("bak")}`;
  } else if (st === "queued") {
    h = `<h2>backup</h2>${pathLine(root)}<span class="qpill"><i></i>Queued</span><div class="sub">Starts as soon as the copy is verified.</div>`;
  } else if (st === "running") {
    el.classList.add("running");
    extra = `<div class="fill" style="width:${S.bakP}%"></div><span class="tagline">Backup</span>
      <button class="corner" data-act="cancel" title="Stop">${ic.x()}</button>`;
    const note = S.runFanOut ? "Reading the card once, writing every drive" : "Reading from the verified copy";
    h = liveBlock("backing up to", root, S.bakP, S.runFanOut ? S.cur : S.curB, note);
  } else if (st === "hold") {
    h = `<h2>backup</h2>${pathLine(root)}<span class="qpill"><i style="background:var(--err)"></i>On hold</span>
      <div class="sub">Resolve the copy errors first.</div>`;
  } else if (st === "error") {
    el.classList.add("error");
    const n = S.failBak;
    const big = n > 0 ? `${n} file${n === 1 ? "" : "s"}<small>failed verification</small>` : `stopped<small>${esc(S.abortReason ?? "")}</small>`;
    h = `<h2>backup incomplete</h2>${pathLine(root)}<div class="pct" style="font-size:44px">${big}</div>
      <div class="row" style="margin-top:8px"><button class="btn sm" data-act="log">${ic.file(13)} Show log</button></div>`;
  } else {
    el.classList.add("done");
    h = `<h2>backed up to</h2>${pathLine(root)}<div class="pct">100%<small>complete</small></div>
      <div class="okring">${ic.check(20, 2.6)}</div>${summaryPill("backed up")}`;
  }
  el.innerHTML = extra + `<div class="content">${h}</div>`;
}

/** Updates the running panels in place (fill, %, current file, speed). */
export function updateLive() {
  if (S.view) return; // a history tab is showing
  const live = (el: HTMLElement, pct: number, cur: { name: string; verify: boolean } | null, rate: string | null) => {
    if (!el.classList.contains("running")) return false;
    const fill = el.querySelector<HTMLElement>(".fill");
    const n = el.querySelector(".pct .n");
    if (!fill || !n) return false;
    fill.style.width = pct + "%";
    n.textContent = Math.floor(pct) + "%";
    el.querySelector(".status")!.innerHTML = statusHtml(cur);
    if (rate !== null) el.querySelector(".rate")!.textContent = rate;
    return true;
  };
  if (copyState() === "running" && !live($("pCopy"), S.copyP, S.cur, rateText())) renderCopy();
  if (bakState() === "running" && !live($("pBak"), S.bakP, S.runFanOut ? S.cur : S.curB, null)) renderBak();
}

/* ---------------------------------------------------------------- strip, grid, footer */

export function renderStrip() {
  const st = running() ? "busy" : S.phase === "done" ? "ok" : S.phase === "error" ? "err" : "";
  const when = S.finishedAt ?? S.startedAt ?? new Date();
  const t = S.finishedAt ? when.toTimeString().slice(0, 8) : "Current";
  const d = when.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  $("tabs").innerHTML =
    historyTabs() +
    `<button class="tab ${S.view ? "" : "active"}" data-act="tab" data-path=""><span class="dot ${st}"></span><span class="t">${t}</span><span class="d">${d}</span></button>`;
  $("menuBtn").innerHTML = ic.menu();
  const kinds = $("kinds");
  kinds.innerHTML = kindsSeg();
  kinds.style.display = kinds.innerHTML ? "" : "none";
  const pb = $("presetBtn");
  pb.innerHTML = presetButton();
  pb.style.display = S.view ? "none" : "";
  // Keep the Current tab in view when there's a long history.
  const tabs = $("tabs");
  tabs.scrollLeft = tabs.scrollWidth;
}

export type Kind = "video" | "stills" | "audio" | "other";
const KINDS: [Kind, string][] = [["video", "Video"], ["stills", "Stills"], ["audio", "Audio"], ["other", "Other"]];

export function kindOf(ext: string): Kind {
  return VIDEO.has(ext) ? "video" : PHOTO.has(ext) ? "stills" : AUDIO.has(ext) ? "audio" : "other";
}

/** Which kind the grid shows: the one picked, else footage first (video, then audio,
 *  then stills), else everything. Only the view changes; every file is still copied. */
export function gridKind(files: FileView[]): Kind | "all" {
  const has = (k: Kind) => files.some((f) => kindOf(f.ext) === k);
  if (S.kind === "all" || (S.kind && has(S.kind))) return S.kind;
  return (["video", "audio", "stills"] as Kind[]).find(has) ?? "all";
}

/** The grid's tiles for `files`: those of the shown kind, plus any that failed. */
export function gridTiles(files: FileView[]): string {
  const k = gridKind(files);
  return files.map((f, i) => (k === "all" || kindOf(f.ext) === k || f.state === "failed" ? thumbHTML(f, i) : "")).join("");
}

/** `files` with the grid's shown kind first, so their thumbnails come first. */
export function byGridKind(files: FileView[]): FileView[] {
  const k = gridKind(files);
  if (k === "all") return files;
  return [...files.filter((f) => kindOf(f.ext) === k), ...files.filter((f) => kindOf(f.ext) !== k)];
}

function kindsSeg(): string {
  const files = shownFiles();
  const counts = KINDS.map(([k, label]) => [k, label, files.filter((f) => kindOf(f.ext) === k).length] as const).filter(([, , n]) => n > 0);
  if (counts.length < 2) return "";
  const cur = gridKind(files);
  const btn = (k: string, label: string, n: number) =>
    `<button class="${cur === k ? "on" : ""}" data-act="kind" data-k="${k}">${label} <span class="n">${n}</span></button>`;
  return counts.map(([k, label, n]) => btn(k, label, n)).join("") + btn("all", "All", files.length);
}

const VIDEO = new Set(["MOV", "MP4", "MXF", "R3D", "BRAW", "NEV", "CRM", "AVI", "MTS", "M2TS", "MKV", "ARI", "MPG", "M4V", "3GP", "INSV", "LRV"]);
const PHOTO = new Set(["JPG", "JPEG", "NEF", "NRW", "CR2", "CR3", "ARW", "DNG", "RAF", "ORF", "RW2", "HEIC", "HEIF", "PNG", "TIF", "TIFF", "PEF", "SRW", "GPR", "IIQ", "3FR"]);
const AUDIO = new Set(["WAV", "BWF", "MP3", "AAC", "M4A", "FLAC", "AIF", "AIFF", "OGG"]);

function thumbArt(f: FileView) {
  const url = thumbUrl(f);
  if (url) return `<div class="img"><img src="${url}" alt=""></div>`;
  const icon = VIDEO.has(f.ext) ? ic.film(30) : PHOTO.has(f.ext) ? ic.image(30) : AUDIO.has(f.ext) ? ic.wave(30) : ic.file(28);
  return `<div class="img">${icon}</div>`;
}

export function thumbHTML(f: FileView, i: number) {
  const cls =
    f.state === "pending" ? "pending"
    : f.state === "copying" ? "active"
    : f.state === "copied" || f.state === "backed" ? "ok"
    : f.state === "backing" ? "bk-active"
    : "fail";
  let st = "";
  if (f.state === "copying") st = f.verify ? "verify" : "copy";
  else if (f.state === "copied") st = ic.check(10, 3);
  else if (f.state === "backing") st = f.verify ? "verify" : "backup";
  else if (f.state === "backed") st = ic.check(10, 3) + ic.check(10, 3);
  else if (f.state === "failed") st = "!";
  return `<div class="thumb ${cls}" data-i="${i}" style="--p:${f.p}%">${thumbArt(f)}<div class="shade"></div>${
    f.ext ? `<span class="ty">${esc(f.ext.slice(0, 5))}</span>` : ""
  }<span class="st">${st}</span><div class="bar"></div><div class="nm">${esc(f.name)}</div></div>`;
}

function banner(icon: string, cls: string, html: string, action = "") {
  return `<div class="banner"><div class="ic ${cls}">${icon}</div><div class="tx">${html}</div>${action}</div>`;
}

export function renderGrid() {
  if (S.view) return; // a history tab is showing
  const w = $("gridwrap");
  if (!S.src) {
    w.innerHTML = `${interruptedBanner()}<div class="empty"><div><div style="display:grid;place-items:center">${ic.cardInsert()}</div>
      <h3>${S.scanning ? "Reading…" : "Waiting for media"}</h3><div>Insert a card or drive, or choose a folder to begin.</div></div></div>`;
    return;
  }
  let banners = "";
  const res = resuming();
  if (S.phase === "ready" && res) {
    const when = new Date(res.started);
    banners += banner(
      ic.refresh(),
      "info",
      `<b>Continuing the offload of ${esc(res.label)}</b> from ${esc(when.toLocaleDateString(undefined, { month: "short", day: "numeric" }))} at ${esc(when.toTimeString().slice(0, 8))}. Files already verified (shown in mint) aren't copied again; nothing in those folders is overwritten.`,
    );
  }
  const conflict = S.dests?.[0];
  if (S.phase === "ready" && conflict?.conflict != null) {
    banners += banner(
      ic.shield(),
      "",
      `<b>${esc(S.src.label)} was already offloaded</b> to <span class="mono">${esc(conflict.requested)}</span>. Offloading again copies into a new <span class="mono">${esc(conflict.root.split(/[\\/]/).pop())}</span> folder — nothing is overwritten.`,
    );
  }
  if (S.phase === "ready" && S.stoppedKept > 0) {
    banners += banner(ic.info(), "info", `<b>Stopped.</b> ${S.stoppedKept} verified file${S.stoppedKept === 1 ? " was" : "s were"} kept. The partial file was removed.`);
  }
  if (S.phase === "error" && S.abortReason) {
    banners += banner(ic.alert(), "bad", `<b>Stopped: ${esc(S.abortReason)}.</b> Verified files were kept.`);
  }
  const problems = S.plan?.problems ?? [];
  if (problems.length) {
    banners += banner(
      ic.alert(),
      "bad",
      `<b>${problems.length} item${problems.length === 1 ? "" : "s"} on this source can't be read</b> (${esc(problems[0].rel)}${problems.length > 1 ? ", …" : ""}). The offload can't be fully verified while they're unreadable.`,
    );
  }
  const renamed = S.plan?.renamed ?? [];
  if (renamed.length && S.phase === "ready") {
    banners += banner(
      ic.info(),
      "info",
      `<b>${renamed.length} name${renamed.length === 1 ? "" : "s"} will change</b> to fit the destination (e.g. <span class="mono">${esc(renamed[0].from)}</span> → <span class="mono">${esc(renamed[0].to)}</span>). The log lists every one.`,
    );
  }
  if (!S.files.length) {
    w.innerHTML = banners + `<div class="empty" style="position:static;padding:60px 0"><div><h3>${S.scanning ? "Reading…" : "No files"}</h3><div>${S.scanning ? "" : "This source has no files to copy."}</div></div></div>`;
    return;
  }
  w.innerHTML = banners + `<div class="grid">${gridTiles(S.files)}</div>`;
}

export function updateThumb(i: number) {
  if (S.view) return; // a history tab is showing
  const old = document.querySelector(`.thumb[data-i="${i}"]`);
  const f = S.files[i];
  if (!old) {
    // Hidden by the kind filter. A failed file is always shown.
    if (f?.state === "failed") renderGrid();
    return;
  }
  const cls = old.className;
  // Same state: just move the progress bar.
  const t = document.createElement("div");
  t.innerHTML = thumbHTML(f, i);
  const fresh = t.firstElementChild as HTMLElement;
  if (fresh.className === cls && old.querySelector(".st")?.innerHTML === fresh.querySelector(".st")?.innerHTML) {
    (old as HTMLElement).style.setProperty("--p", f.p + "%");
  } else {
    old.replaceWith(fresh);
  }
}

export function flashThumb(i: number, label: string) {
  if (S.view) return; // a history tab is showing
  const el = document.querySelector(`.thumb[data-i="${i}"]`);
  if (!el) return;
  const d = document.createElement("div");
  d.className = "flash";
  d.innerHTML = `<div><span>${esc(label)}</span><i>${ic.check(18, 3)}</i></div>`;
  el.appendChild(d);
  setTimeout(() => d.remove(), 1000);
}

export function renderFoot() {
  if (S.view) return; // a history tab is showing
  $("fl").innerHTML = S.src ? `<span style="color:var(--tx2)">${esc(S.src.label)}</span> · ${esc(S.src.media)}` : "No source";
  $("fc").textContent = S.src && S.plan ? `${S.files.length} file${S.files.length === 1 ? "" : "s"} · ${fmtSize(totalBytes())}` : "";
  $("fr").innerHTML = `<span>${ic.shield(12)} xxHash64${S.settings.md5 ? " + MD5" : ""}</span><span>${S.settings.mhl ? "MHL on" : "MHL off"}</span><span>${S.settings.fanOut ? "Fan-out" : "Sequential"}</span>`;
}

export function render() {
  if (S.view) {
    renderStrip();
    renderHistory();
    return;
  }
  renderSrc();
  renderCopy();
  renderBak();
  renderStrip();
  renderGrid();
  renderFoot();
}
