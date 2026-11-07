// Thumbnails: asked for in the background, cached by file identity (name, size, date) so
// a card file and its verified copy share one image, and patched into tiles as they come.

import { thumbsApi } from "./api";
import { sep, shownFiles, type FileView } from "./state";

const urls = new Map<string, string | null>();
let generation = 0;

export const identity = (f: FileView) => `${f.name}|${f.size}|${f.modified ?? 0}`;
export const thumbUrl = (f: FileView) => urls.get(identity(f)) ?? null;

/** Asks for thumbnails of `files` under `root`, replacing any earlier request. */
export function requestThumbs(files: FileView[], root: string, useDest: boolean) {
  generation += 1;
  const base = root.replace(/[\/]+$/, "");
  const items = files
    .filter((f) => !urls.has(identity(f)))
    .map((f) => ({ key: identity(f), path: base + sep + (useDest ? f.destRel : f.srcRel).split("/").join(sep) }));
  thumbsApi.request(generation, items).catch(() => {});
}

/** Stops thumbnail work (while the card is being read). */
export function stopThumbs() {
  generation += 1;
  thumbsApi.request(generation, []).catch(() => {});
}

export function initThumbs() {
  thumbsApi.onThumb((t) => {
    if (t.generation !== generation) return;
    urls.set(t.key, t.url);
    if (!t.url) return;
    shownFiles().forEach((f, i) => {
      if (identity(f) !== t.key) return;
      const img = document.querySelector(`.thumb[data-i="${i}"] .img`);
      if (img) img.innerHTML = `<img src="${t.url}" alt="">`;
    });
  });
}
