// The thumbnail under the pointer, and where the pointer is, for the file tooltip.

import type { FileView } from "../../state.svelte.ts";

export interface Hover {
  file: FileView;
  x: number;
  y: number;
}

let current = $state.raw<Hover | null>(null);

export const hovered = () => current;

/** The pointer is over `file`'s tile (entering it, or moving within it). */
export function hoverFile(file: FileView, e: MouseEvent) {
  current = { file, x: e.clientX, y: e.clientY };
}

/** The pointer left the tile, or the tile went away while under it. */
export function clearHover() {
  current = null;
}
