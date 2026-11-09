// Thumbnails: asked for in the background, cached by file identity (name, size, date) so
// a card file and its verified copy share one image. Tiles update as they come.

import { SvelteMap } from "svelte/reactivity";
import { thumbsApi } from "./api";
import { sep, type FileView } from "./state.svelte.ts";

const urls = new SvelteMap<string, string | null>();
let generation = 0;

export const identity = (f: FileView) => `${f.name}|${f.size}|${f.modified ?? 0}`;
export const thumbUrl = (f: FileView) => urls.get(identity(f)) ?? null;

/** Asks for thumbnails of `files` under `root`, replacing any earlier request. */
export function requestThumbs(files: FileView[], root: string, useDest: boolean) {
  generation += 1;
  // Trim either separator: a drive root like `H:\` would otherwise give `H:\\DCIM\…`,
  // which the file system accepts but the Windows Shell doesn't.
  const base = root.replace(/[\\/]+$/, "");
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
  });
}
