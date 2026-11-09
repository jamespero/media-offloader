// Mouse input on thumbnails: the tooltip, and double-click to show in the file manager.

import { api } from "./api";
import { $, esc, fmtSize } from "./render";
import { S, fileManager, sep, shownFiles } from "./state";

/** Double-click a thumbnail: show the verified copy (or the original) in the file manager. */
function onDoubleClick(e: MouseEvent) {
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
}

/** Tooltip on thumbnails. */
function onMouseMove(e: MouseEvent, tip: HTMLElement) {
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
}

export function initThumbInput() {
  const tip = $("tip");
  document.addEventListener("dblclick", onDoubleClick);
  document.addEventListener("mousemove", (e) => onMouseMove(e, tip));
}
