// Menus, dialogs, the settings sheet, the log viewer and toasts.

import { api } from "./api";
import { ic, type IconName } from "./icons";
import { $, esc, fmtSize } from "./render";
import { S, fileManager, running, tokenContext } from "./state";
import { DATE_FMTS, DAY_FMTS, TOKEN_TYPES, fmtDate, fmtDay, tokenInfo, tokenText } from "./tokens";
import { namingPane } from "./presets";

/* ---------------------------------------------------------------- menus */

export function closeMenus() {
  document.querySelectorAll(".menu").forEach((m) => m.remove());
}

export function openMenu(anchor: Element, html: string, align: "left" | "right" = "left") {
  closeMenus();
  const m = document.createElement("div");
  m.className = "menu";
  m.innerHTML = html;
  document.body.appendChild(m);
  const r = anchor.getBoundingClientRect();
  let x = align === "right" ? r.right - m.offsetWidth : r.left;
  let y = r.bottom + 6;
  if (y + m.offsetHeight > innerHeight - 8) y = Math.max(8, r.top - m.offsetHeight - 6);
  x = Math.max(8, Math.min(x, innerWidth - m.offsetWidth - 8));
  m.style.left = x + "px";
  m.style.top = y + "px";
  return m;
}

export const mi = (icon: IconName | null, label: string, act: string, extra = "", attrs = "") =>
  `<button class="mi" data-act="${act}" ${attrs}>${icon ? ic[icon](15) : '<span style="width:15px"></span>'}<span>${label}</span>${extra}</button>`;

const mod = navigator.userAgent.includes("Mac") ? "⌘" : "Ctrl+";

export function mainMenu(a: Element) {
  const log = S.report?.logPath;
  const copyRoot = S.phase === "done" || S.phase === "error" ? S.dests?.[0]?.root : null;
  openMenu(
    a,
    `${mi("newf", "New Offload", "new", `<span class="kb">${mod}N</span>`, running() ? "disabled" : "")}
     ${mi("refresh", "Re-verify a Folder…", "reverify", `<span class="kb">${mod}R</span>`)}
     <div class="sep"></div>
     ${mi("file", "Open Log for This Offload", "log", `<span class="kb">${mod}L</span>`, log ? "" : "disabled")}
     ${mi("folder", "Show Logs Folder", "logsfolder")}
     ${mi("reveal", `${navigator.userAgent.includes("Mac") ? "Reveal Copy in" : "Show Copy in"} ${fileManager}`, "revealcopy", "", copyRoot ? "" : "disabled")}
     <div class="sep"></div>
     ${mi("gear", "Settings…", "settings", `<span class="kb">${mod},</span>`)}
     ${mi("info", "About Media Offloader", "about")}`,
    "right",
  );
}

export function srcMenu(a: Element) {
  const vols = S.volumes;
  const list = vols.length
    ? vols
        .map((v) =>
          mi(
            v.removable ? "disk" : "disk",
            esc(v.label),
            "pick",
            `<span class="sm">${fmtSize(v.total)}</span>`,
            `data-path="${esc(v.path)}" title="${esc(v.path)}"`,
          ),
        )
        .join("")
    : `<div class="mnote">No cards or drives found.</div>`;
  openMenu(a, `<div class="mh">Connected media</div>${list}<div class="sep"></div>${mi("folder", "Choose Folder…", "pickfolder")}`);
}

export function destMenu(a: Element, w: "copy" | "bak") {
  const recent = (w === "copy" ? S.settings.recentCopy : S.settings.recentBak).slice(0, 5);
  const items = recent.map((r) =>
    mi("folder", `<span style="font-family:var(--f-mono);font-size:12px">${esc(r)}</span>`, "setdest", "", `data-w="${w}" data-v="${esc(r)}"`),
  );
  openMenu(
    a,
    `${items.length ? `<div class="mh">Recent</div>${items.join("")}<div class="sep"></div>` : ""}${mi("reveal", "Browse…", "browsedest", "", `data-w="${w}"`)}`,
  );
}

export function addTokenMenu(a: Element) {
  openMenu(
    a,
    TOKEN_TYPES.map((t) => ("sep" in t ? '<div class="sep"></div>' : mi(t.icon, t.label, "addtok", t.isNew ? '<span class="sm">new</span>' : "", `data-t="${t.t}"`))).join(""),
  );
}

export function chipMenu(a: Element, i: number, onEdit: () => void) {
  const tk = S.settings.tokens[i];
  const ctx = tokenContext();
  let h = "";
  if (tk.t === "now" || tk.t === "shot") {
    const sample = tk.t === "now" ? ctx.now : (ctx.shotDate ?? ctx.now);
    h =
      `<div class="mh">${tk.t === "now" ? "Now: current date/time" : "Shot: capture date/time"}</div>` +
      DATE_FMTS.map(
        (f) =>
          `<button class="mi" data-act="fmt" data-i="${i}" data-f="${f}"><span style="width:15px">${(tk.fmt ?? "YY-MM-DD") === f ? ic.check(14) : ""}</span><span>${f}</span><span class="sm">${esc(fmtDate(sample, f))}</span></button>`,
      ).join("");
  } else if (tk.t === "day") {
    const sample = ctx.shotDate ?? ctx.now;
    h =
      `<div class="mh">Shoot day</div>` +
      numberRow("Day", S.settings.shootDay, "Set by hand; it doesn't count up on its own.") +
      `<div class="sep"></div><div class="mh">Format</div>` +
      DAY_FMTS.map(
        (f) =>
          `<button class="mi" data-act="fmt" data-i="${i}" data-f="${f}"><span style="width:15px">${(tk.fmt ?? "Day01") === f ? ic.check(14) : ""}</span><span>${f}</span><span class="sm">${esc(fmtDay(S.settings.shootDay, f, sample))}</span></button>`,
      ).join("");
  } else if (tk.t === "text" || tk.t === "project") {
    h = `<div class="mh">${tk.t === "text" ? "Custom text" : "Project name"}</div><div style="padding:4px 6px 6px"><input class="tin" id="txtIn" value="${esc(tk.v ?? "")}" spellcheck="false"></div>`;
  } else if (tk.t === "reel") {
    h =
      `<div class="mh">Reel letter</div>` +
      ["A", "B", "C", "D"]
        .map((l) => mi("tag", `${l}${String(ctx.reelNo(l)).padStart(3, "0")}, ${l}${String(ctx.reelNo(l) + 1).padStart(3, "0")}…`, "reel", (tk.v || "A") === l ? `<span class="ck">${ic.check(14)}</span>` : "", `data-i="${i}" data-v="${l}"`))
        .join("") +
      `<div class="sep"></div>` +
      numberRow(`Next ${tk.v || "A"} roll`, ctx.reelNo(tk.v || "A"), "Counts up after each completed offload. Change it to match the card's label.");
  } else {
    h = `<div class="mh">${tokenInfo(tk.t).label}</div><div class="mnote">Resolves to <span style="font-family:var(--f-mono);color:var(--tx2)">${esc(tokenText(tk, ctx))}</span></div>`;
  }
  const n = S.settings.tokens.length;
  h += `<div class="sep"></div>${i > 0 ? mi("chevL", "Move Left", "mvl", "", `data-i="${i}"`) : ""}${i < n - 1 ? mi("chevR", "Move Right", "mvr", "", `data-i="${i}"`) : ""}${mi("trash", "Remove", "rmtok", "", `data-i="${i}"`)}`;
  const m = openMenu(a, h);
  const num = m.querySelector<HTMLInputElement>("#numIn");
  if (num) {
    num.oninput = () => {
      const n = Math.trunc(Number(num.value));
      if (!Number.isFinite(n) || n < 1 || n > 999) return;
      if (tk.t === "day") S.settings.shootDay = n;
      else S.settings.reels[tk.v || "A"] = n;
      onEdit();
    };
    num.onkeydown = (e) => {
      if (e.key === "Enter") closeMenus();
    };
  }
  const inp = m.querySelector<HTMLInputElement>("#txtIn");
  if (inp) {
    inp.focus();
    inp.select();
    inp.oninput = () => {
      tk.v = inp.value;
      onEdit();
    };
    inp.onkeydown = (e) => {
      if (e.key === "Enter") closeMenus();
    };
  }
}

/** A labelled number field for the chip menus (shoot day, next roll). */
function numberRow(label: string, value: number, note: string) {
  return `<div class="row" style="padding:4px 10px 6px;gap:8px;justify-content:space-between"><span style="font-size:12px;color:var(--tx2)">${esc(label)}</span><input class="tin" id="numIn" type="number" min="1" max="999" value="${value}" style="width:76px"></div><div class="mnote">${esc(note)}</div>`;
}

/* ---------------------------------------------------------------- overlays */

export function overlay(html: string) {
  closeOverlay();
  const o = document.createElement("div");
  o.className = "scrim";
  o.id = "scrim";
  o.innerHTML = html;
  $("app").appendChild(o);
  o.addEventListener("mousedown", (e) => {
    if (e.target === o) closeOverlay();
  });
  return o;
}

let onOverlayClose: (() => void) | null = null;

/** Runs `f` once when the current overlay closes (Escape, click outside, Close). */
export function whenOverlayCloses(f: () => void) {
  onOverlayClose = f;
}

export function closeOverlay() {
  document.getElementById("scrim")?.remove();
  const f = onOverlayClose;
  onOverlayClose = null;
  f?.();
}

export function confirmStop() {
  const done = S.files.filter((f) => f.state === "copied" || f.state === "backed").length;
  overlay(`<div class="dialog"><h3>Stop this offload?</h3>
    <p>${done} of ${S.files.length} files are already copied and verified. Those stay where they are.</p>
    <p>The file in progress will be removed so nothing half-written is left behind.</p>
    <div class="acts"><button class="btn" data-act="closeov">Keep going</button><button class="btn danger" data-act="dostop">Stop</button></div></div>`);
}

export async function showLog(path: string) {
  let text: string;
  try {
    text = await api.readText(path);
  } catch (e) {
    text = `Couldn't open the log: ${e}`;
  }
  const lines = text.split(/\r?\n/).map((l) => {
    const e = esc(l);
    if (/^OK /.test(l) || /^Result +All files verified/.test(l)) return `<span class="ok">${e}</span>`;
    if (/^(FAIL|UNREADABLE) /.test(l) || /^Result +(?!All)/.test(l)) return `<span class="er">${e}</span>`;
    if (/^(==|Media Offloader)/.test(l)) return `<span class="hd">${e}</span>`;
    return e;
  });
  overlay(`<div class="logbox"><header>${ic.file()}<b>Transfer log</b>
    <button class="btn sm" data-act="revealpath" data-path="${esc(path)}">${ic.reveal(13)} Show in ${fileManager}</button>
    <button class="iconbtn" data-act="closeov" aria-label="Close">${ic.x()}</button></header><pre>${lines.join("\n")}</pre></div>`);
}

export function about() {
  overlay(`<div class="dialog"><h3>Media Offloader</h3>
    <p>Version 0.1.0 · open source under MIT or Apache-2.0</p>
    <p>Copies camera cards with checksum verification (xxHash64, optional MD5), re-reading every copy from disk, and never overwrites anything.</p>
    <p style="color:var(--tx3)">Fonts: Outfit, Inter and JetBrains Mono, under the SIL Open Font License.</p>
    <div class="acts"><button class="btn" data-act="closeov">Close</button></div></div>`);
}

const SETTINGS_TABS: [string, string, IconName][] = [
  ["general", "General", "gear"],
  ["verify", "Verification", "shield"],
  ["transfers", "Transfers", "layers"],
  ["naming", "Naming & Presets", "star"],
  ["logs", "Logs", "file"],
];

export async function settings(tab = "general") {
  const s = S.settings;
  const t = (on: boolean, key: string, lock = false) =>
    `<button class="tog ${on ? "on" : ""} ${lock ? "lock" : ""}" ${lock ? "" : `data-act="tog" data-k="${key}"`} aria-pressed="${on}"></button>`;
  const set = (b: string, sub: string, ctl: string) => `<div class="set"><div class="l"><b>${b}</b><span>${sub}</span></div>${ctl}</div>`;
  const busy = running();
  const logDir = (await api.logDir()) ?? "";
  const P: Record<string, string> = {
    general: `<h5>general</h5><p class="intro">How Media Offloader reacts when media shows up.</p>
      ${set("Auto-select inserted media", "When a card or drive is connected, make it the source. Folders are never auto-selected.", t(s.autoSelect, "autoSelect"))}
      ${set("Copy hidden system files", "Include .Trashes, .Spotlight-V100, System Volume Information. Off still copies every camera file and folder.", t(s.includeSystemFiles, "includeSystemFiles"))}
      ${set("Eject source when verified", "Unmount the card as soon as its copy is verified, so it is safe to pull. The backup carries on from the copy.", t(s.ejectWhenVerified, "ejectWhenVerified"))}
      ${set("Keep the computer awake", "Prevent sleep while a transfer is running.", t(s.keepAwake, "keepAwake"))}`,
    verify: `<h5>verification</h5><p class="intro">Every file is read back from the destination and compared to the source.</p>
      ${set("Checksum", "xxHash64 is fast and is what ASC MHL uses. Add MD5 when production or insurance asks for it.", `<div class="seg"><button class="${s.md5 ? "" : "on"}" data-act="md5" data-v="0" ${busy ? "disabled" : ""}>xxHash64</button><button class="${s.md5 ? "on" : ""}" data-act="md5" data-v="1" ${busy ? "disabled" : ""}>xxHash64 + MD5</button></div>`)}
      ${set("Write ASC MHL manifest", "Saves an ascmhl folder next to the footage so any copy can be re-verified later, here or in any MHL tool.", t(s.mhl, "mhl"))}
      ${set("Write .md5 checksum list", "Classic \"hash  path\" file (checksums.md5) next to the footage. Requires MD5.", t(s.md5List && s.md5, "md5List", !s.md5))}
      ${set("Verify from disk, not cache", "Reads bypass the OS cache so verification checks what is actually on the drive. Always on.", t(true, "", true))}`,
    transfers: `<h5>transfers</h5><p class="intro">How copy and backup are scheduled.</p>
      ${set("Copy mode", "Sequential: copy, verify, free the card, then back up from the copy (classic). Fan-out: read the card once and write every drive at the same time.", `<div class="seg"><button class="${s.fanOut ? "" : "on"}" data-act="fan" data-v="0" ${busy ? "disabled" : ""}>Sequential</button><button class="${s.fanOut ? "on" : ""}" data-act="fan" data-v="1" ${busy ? "disabled" : ""}>Fan-out</button></div>`)}
      ${set("When the destination already exists", "Never overwrite. Repeat offloads go into copy-conflicted, copy-conflicted-1, …", `<span class="meta">Always on</span>`)}`,
    naming: namingPane(set),
    logs: `<h5>logs</h5><p class="intro">A plain-text log is kept for every offload: each file's checksums, what was skipped or renamed, and how it ended.</p>
      ${set("Log location", `<span class="mono">${esc(logDir)}</span>`, `<button class="btn sm" data-act="logsfolder">${ic.reveal(13)} Show</button>`)}`,
  };
  overlay(`<div class="modal"><div class="nav"><h4>settings</h4>${SETTINGS_TABS.map(([k, l, i]) => `<button class="mi ${k === tab ? "on" : ""}" data-act="stab" data-tab="${k}">${ic[i](15)}<span>${l}</span></button>`).join("")}</div>
    <div class="pane"><button class="iconbtn closex" data-act="closeov" aria-label="Close">${ic.x()}</button>${P[tab]}</div></div>`);
}

export function currentSettingsTab() {
  return document.querySelector<HTMLElement>(".nav .mi.on")?.dataset.tab;
}

/* ---------------------------------------------------------------- toast */

export function toast(ok: boolean, title: string, body: string) {
  document.querySelectorAll(".toast").forEach((t) => t.remove());
  const t = document.createElement("div");
  t.className = "toast";
  t.innerHTML = `<div class="ic ${ok ? "" : "bad"}">${ok ? ic.check(18, 2.6) : ic.alert(18)}</div><div><b>${esc(title)}</b><span>${esc(body)}</span></div>`;
  $("app").appendChild(t);
  setTimeout(() => t.remove(), 5000);
}
