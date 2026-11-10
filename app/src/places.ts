// Showing files and folders in the file manager, and opening links.

import { api } from "./api";
import { toast } from "./components/Toast/toastState.svelte.ts";
import { S, sep, type FileView } from "./state.svelte.ts";

/** Shows `path` selected in Explorer or Finder. */
export function reveal(path: string) {
  api.reveal(path).catch(() => {});
}

export async function showLogsFolder() {
  const d = await api.logDir();
  if (d) api.openFolder(d).catch((e) => toast(false, "Couldn't open the logs folder", String(e)));
}

/** Shows this offload's copy, once it has one. */
export function revealCopy() {
  if (S.dests?.[0]) reveal(S.dests[0].root);
}

export function openRepo() {
  api.openRepo().catch(() => {});
}

/** Shows a file from the grid: its verified copy, or the original when it isn't copied yet. */
export function revealFile(f: FileView) {
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
  if (path) reveal(path);
}
