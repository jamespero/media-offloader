// Which thumbnails to ask for, given what's on screen.

import { byGridKind } from "./render";
import { S, running } from "./state";
import { requestThumbs } from "./thumbs";

/** Thumbnails for the Current tab: from the card before a run, from the verified copy after. */
export function thumbsForCurrent() {
  if (!S.src || running()) return;
  const copy = S.dests?.[0]?.root;
  if ((S.phase === "done" || S.phase === "error") && copy) requestThumbs(byGridKind(S.files), copy, true);
  else if (S.phase === "ready") requestThumbs(byGridKind(S.files), S.src.path, false);
}

export function thumbsForHistory() {
  const d = S.detail;
  if (!d) return;
  requestThumbs(byGridKind(S.histFiles), d.item.roots[0] ?? d.item.source, !!d.item.roots[0]);
}
