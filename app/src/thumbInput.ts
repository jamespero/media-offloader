// Double-click a thumbnail to show it in the file manager.

import { api } from "./api";
import { S, sep, shownFiles } from "./state.svelte.ts";

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

export function initThumbInput() {
  document.addEventListener("dblclick", onDoubleClick);
}
