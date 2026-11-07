// "Re-verify a Folder…": re-checks a copy against its ASC MHL history (or .md5 list).
// Read-only; the window updates in place, and closing it stops a running check.

import { open } from "@tauri-apps/plugin-dialog";
import { reverifyApi, type CheckStatus, type ReverifyReport, type ManifestSource } from "./api";
import { ic } from "./icons";
import { closeOverlay, overlay, showLog, whenOverlayCloses } from "./menus";
import { esc, fmtSize } from "./render";
import { fileManager } from "./state";

const R = {
  path: "",
  running: false,
  files: 0,
  bytes: 0,
  done: 0,
  checked: 0,
  failed: 0,
  current: "",
  source: null as ManifestSource | null,
  report: null as ReverifyReport | null,
  error: null as string | null,
};

export function reverifyIntro() {
  overlay(`<div class="dialog"><h3>Re-verify a folder</h3>
    <p>Pick a folder that has an <span class="mono">ascmhl</span> manifest (or a .md5 list). Every file is re-read from disk and compared against the checksums recorded when it was offloaded.</p>
    <p style="color:var(--tx3)">Use it to check an archive drive, or to find out which copy is good when two disagree. Nothing in the folder is changed.</p>
    <div class="acts"><button class="btn" data-act="closeov">Cancel</button><button class="btn primary" data-act="rvchoose" style="height:30px">Choose Folder…</button></div></div>`);
}

export async function chooseAndReverify() {
  const p = await open({ directory: true, title: "Re-verify a folder" });
  if (typeof p !== "string") return;
  Object.assign(R, { path: p, running: true, files: 0, bytes: 0, done: 0, checked: 0, failed: 0, current: "", source: null, report: null, error: null });
  overlay(`<div class="logbox rv"><header>${ic.shield()}<b>Re-verify</b><span class="rvpath mono" title="${esc(p)}">${esc(p)}</span>
    <button class="iconbtn" data-act="closeov" aria-label="Close">${ic.x()}</button></header><div class="rvbody" id="rvbody"></div></div>`);
  whenOverlayCloses(() => {
    if (R.running) reverifyApi.cancel();
  });
  draw();
  try {
    await reverifyApi.start(p);
  } catch (e) {
    R.running = false;
    R.error = String(e);
    draw();
  }
}

function sourceText(s: ManifestSource | null) {
  if (!s) return "";
  return s.kind === "mhl" ? `ASC MHL history (${s.generations} generation${s.generations === 1 ? "" : "s"})` : esc(s.file);
}

function statusText(s: CheckStatus): string {
  switch (s.state) {
    case "missing":
      return "missing";
    case "sizeDiffers":
      return `size is ${s.actual} bytes, the manifest says ${s.expected}`;
    case "mismatch":
      return `${s.format} doesn't match (${s.actual.slice(0, 16)}…, expected ${s.expected.slice(0, 16)}…)`;
    case "readError":
      return `couldn't read: ${s.message}`;
    default:
      return "";
  }
}

function draw() {
  const el = document.getElementById("rvbody");
  if (!el) return;
  if (R.error) {
    el.innerHTML = `<div class="rvres bad">${ic.alert(18)}<span>${esc(R.error)}</span></div>
      <div class="acts"><button class="btn" data-act="closeov">Close</button></div>`;
    return;
  }
  if (R.running || !R.report) {
    const pct = R.bytes ? (R.done / R.bytes) * 100 : 0;
    el.innerHTML = `<div class="rvbar"><div class="fill" style="width:${pct}%"></div></div>
      <div class="rvstat"><span class="pct-s">${Math.floor(pct)}%</span><span class="f mono">${esc(R.current)}</span></div>
      <div class="rvnote">${R.files ? `${R.checked} of ${R.files} files · ${fmtSize(R.bytes)} · checking against ${sourceText(R.source)}` : "Reading the manifest…"}${R.failed ? ` · <span style="color:var(--err)">${R.failed} not matching so far</span>` : ""}</div>
      <div class="acts"><button class="btn" data-act="closeov">Stop</button></div>`;
    return;
  }
  const r = R.report;
  const bad = r.files.filter((f) => f.status.state !== "verified" && f.status.state !== "pending");
  const head =
    r.outcome.kind === "verified"
      ? `<div class="rvres ok">${ic.check(18, 2.6)}<span>All ${r.files.length} files match their recorded checksums.</span></div>`
      : r.outcome.kind === "failed"
        ? `<div class="rvres bad">${ic.alert(18)}<span>${r.outcome.files === 1 ? "1 file doesn't" : `${r.outcome.files} files don't`} match their recorded checksums.</span></div>`
        : `<div class="rvres">${ic.info(18)}<span>Stopped after ${R.checked} of ${r.files.length} files.</span></div>`;
  const lines = [
    ...r.manifestProblems.map((p) => `<div class="er">Manifest: ${esc(p)}</div>`),
    ...bad.map((f) => `<div class="er">FAIL  ${esc(f.path)} — ${esc(statusText(f.status))}</div>`),
    ...r.notListed.slice(0, 200).map((p) => `<div>new   ${esc(p)} — not in any manifest</div>`),
  ];
  el.innerHTML = `${head}
    ${lines.length ? `<div class="rvlist">${lines.join("")}</div>` : ""}
    <div class="rvnote">Checked against ${sourceText(r.source)} · every file read from disk, bypassing the OS cache.</div>
    <div class="acts">
      ${r.logPath ? `<button class="btn" data-act="rvlog">${ic.file(13)} Show log</button>` : ""}
      <button class="btn" data-act="revealpath" data-path="${esc(r.root)}">${ic.reveal(13)} Show in ${fileManager}</button>
      <button class="btn" data-act="closeov">Close</button></div>`;
}

export function initReverify() {
  reverifyApi.onEvent((e) => {
    switch (e.type) {
      case "started":
        R.files = e.files;
        R.bytes = e.bytes;
        R.source = e.source;
        break;
      case "progress":
        R.done = e.done;
        break;
      case "fileStarted":
        R.current = e.path;
        break;
      case "fileFinished":
        R.checked += 1;
        if (e.status.state !== "verified") R.failed += 1;
        break;
      case "finished":
        break;
    }
    draw();
  });
  reverifyApi.onReport((r) => {
    R.running = false;
    R.report = r;
    draw();
  });
  reverifyApi.onError((msg) => {
    R.running = false;
    R.error = msg;
    draw();
  });
}

export function reverifyLog() {
  if (R.report?.logPath) {
    const path = R.report.logPath;
    closeOverlay();
    showLog(path);
  }
}
