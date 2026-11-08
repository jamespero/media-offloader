// History: one tab per past offload (from its journal), a read-only view of each, and
// Resume / Retry for offloads that didn't finish.

import { setFooter } from "./components/Footer/footerState.svelte.ts";
import { historyApi, type HistoryDetail, type HistoryItem } from "./api";
import { ic } from "./icons";
import { $, esc, fmtSize, gridTiles } from "./render";
import { S, fileView, running, type FileView } from "./state";

const TABS = 8;

export async function refreshHistory() {
  try {
    S.history = await historyApi.list(30);
  } catch {
    S.history = [];
  }
}

const p2 = (n: number) => String(n).padStart(2, "0");
const clock = (ms: number) => {
  const d = new Date(ms);
  return `${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`;
};
const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { month: "short", day: "numeric" });

export function itemState(it: HistoryItem): "ok" | "err" | "" {
  if (!it.outcome) return "err"; // didn't finish
  if (it.outcome.kind === "verified") return "ok";
  if (it.outcome.kind === "cancelled") return "";
  return "err";
}

/** Tabs for recent offloads, oldest on the left, ending with Current. */
export function historyTabs(): string {
  const items = S.history.filter((h) => h.path !== S.currentJournal).slice(0, TABS).reverse();
  return items
    .map(
      (h) => `<button class="tab ${S.view === h.path ? "active" : ""}" data-act="tab" data-path="${esc(h.path)}" title="${esc(h.label)}">
        <span class="dot ${itemState(h)}"></span><span class="t">${clock(h.started)}</span><span class="d">${day(h.started)}</span>
        <span class="x" data-act="forget" data-path="${esc(h.path)}" title="Remove from history">${ic.x(11)}</span></button>`,
    )
    .join("");
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

function statusChip(it: HistoryItem) {
  const o = it.outcome;
  if (!o) return `<span class="bad">${ic.alert(13)} Didn't finish</span>`;
  switch (o.kind) {
    case "verified":
      return `<span class="safe">${ic.check(13)} Copied &amp; verified</span>`;
    case "failed":
      return `<span class="bad">${ic.alert(13)} ${o.files} file${o.files === 1 ? "" : "s"} failed</span>`;
    case "cancelled":
      return `<span class="qpill"><i></i>Stopped</span>`;
    case "aborted":
      return `<span class="bad">${ic.alert(13)} Stopped</span>`;
  }
}

function pathLine(p: string) {
  return `<div class="path" title="${esc(p)}"><bdi>${esc(p)}</bdi></div>`;
}

function destPanel(el: HTMLElement, it: HistoryItem, d: number) {
  el.className = "panel";
  const backup = d > 0;
  const root = it.roots[d];
  if (root === undefined) {
    el.classList.add("dim");
    el.innerHTML = `<div class="content"><h2>backup</h2><div class="sub">No backup in this offload.</div></div>`;
    return;
  }
  const done = it.verified[d] ?? 0;
  if (done === it.files) {
    el.classList.add("done");
    el.innerHTML = `<div class="content"><h2>${backup ? "backed up to" : "copied to"}</h2>${pathLine(root)}
      <div class="pct">100%<small>complete</small></div><div class="okring">${ic.check(20, 2.6)}</div>
      <button class="summary" data-act="histlog" title="Open the log for this offload">${fmtSize(it.bytes)} in ${it.files} file${it.files === 1 ? "" : "s"} · ${backup ? "backed up" : "copied"} &amp; checksum verified ${ic.info(12)}</button></div>`;
    return;
  }
  const pct = it.files ? Math.floor((done / it.files) * 100) : 0;
  const stopped = !it.outcome || it.outcome.kind === "cancelled";
  if (stopped) {
    el.innerHTML = `<div class="content"><h2>${backup ? "backup" : "copy"}</h2>${pathLine(root)}
      <div class="pct" style="font-size:44px">${pct}%<small>${done} of ${it.files} files verified</small></div>
      <span class="qpill"><i ${it.outcome ? "" : 'style="background:var(--err)"'}></i>${it.outcome ? "Stopped" : "Didn't finish"}</span></div>`;
    return;
  }
  el.classList.add("error");
  const failed = it.failed[d] ?? 0;
  const big = failed > 0 ? `${failed} file${failed === 1 ? "" : "s"}<small>failed verification</small>` : `${pct}%<small>${done} of ${it.files} files verified</small>`;
  el.innerHTML = `<div class="content"><h2>${backup ? "backup incomplete" : "copy incomplete"}</h2>${pathLine(root)}
    <div class="pct" style="font-size:44px">${big}</div>
    <div class="row" style="margin-top:8px"><button class="btn sm" data-act="histlog">${ic.file(13)} Show log</button></div></div>`;
}

/** Renders the read-only view of the history tab being shown. */
export function renderHistory() {
  const d = S.detail;
  const w = $("gridwrap");
  if (!d) {
    w.innerHTML = `<div class="empty"><h3>Loading…</h3></div>`;
    return;
  }
  const it = d.item;
  const icon = it.sourceKind === "folder" ? ic.folder() : ic.card();
  let action = "";
  if (it.resumable) {
    action = it.sourcePresent
      ? `<button class="btn primary" data-act="resume" data-path="${esc(it.path)}" style="margin-top:4px" ${running() ? "disabled" : ""}>${resumeLabel(it)}</button>`
      : `<div class="sub">Connect ${esc(it.label)} to ${resumeLabel(it).toLowerCase()}.</div>`;
  }
  $("pSrc").className = "panel src";
  $("pSrc").innerHTML = `<div class="content"><div class="srcicon">${icon}</div><h2>source</h2>
    <div class="vol" style="cursor:default"><span>${esc(it.label)}</span></div><div class="meta">${day(it.started)} · ${clock(it.started)}</div>
    ${statusChip(it)}${action}</div>`;
  destPanel($("pCopy"), it, 0);
  destPanel($("pBak"), it, 1);

  const when = `${new Date(it.started).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })} at ${clock(it.started)}`;
  const runs = it.runs > 1 ? ` in ${it.runs} runs` : "";
  const note = it.resumable
    ? ` ${resumeLabel(it)} continues in the same folder${it.roots.length > 1 ? "s" : ""}; files already verified aren't copied again.`
    : "";
  const ic2 = it.outcome?.kind === "verified" ? `<div class="ic">${ic.shield()}</div>` : `<div class="ic ${it.outcome?.kind === "cancelled" ? "info" : "bad"}">${ic.info()}</div>`;
  w.innerHTML = `<div class="banner">${ic2}<div class="tx"><b>${esc(it.label)}</b>, offloaded ${esc(when)}${runs} · ${it.files} files, ${fmtSize(it.bytes)}.${esc(note)}</div></div>
    <div class="grid">${gridTiles(S.histFiles)}</div>`;

  setFooter({
    title: it.label,
    detail: day(it.started),
    count: `${it.files} file${it.files === 1 ? "" : "s"} · ${fmtSize(it.bytes)}`,
    items: [{ icon: "file", text: "History" }],
  });
}

/** Banner for the Current tab when the last offload didn't finish. */
export function interruptedBanner(): string {
  const last = S.history[0];
  if (!last || last.outcome || running() || S.src || last.path === S.currentJournal) return "";
  const btn = last.sourcePresent
    ? `<button class="btn sm" data-act="resume" data-path="${esc(last.path)}">Resume</button>`
    : `<button class="btn sm" data-act="tab" data-path="${esc(last.path)}">View</button>`;
  return `<div class="banner"><div class="ic bad">${ic.alert()}</div><div class="tx"><b>The offload of ${esc(last.label)} from ${day(last.started)} at ${clock(last.started)} didn't finish.</b> ${
    last.sourcePresent ? "Resume picks up where it stopped." : `Connect ${esc(last.label)} to resume it.`
  }</div>${btn}</div>`;
}
