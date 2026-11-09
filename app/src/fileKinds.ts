// Sorting files by kind for the grid's filter.

import { S, type FileView } from "./state.svelte.ts";

export type Kind = "video" | "stills" | "audio" | "other";

export const KINDS: [Kind, string][] = [
  ["video", "Video"],
  ["stills", "Stills"],
  ["audio", "Audio"],
  ["other", "Other"],
];

const VIDEO = new Set(["MOV", "MP4", "MXF", "R3D", "BRAW", "NEV", "CRM", "AVI", "MTS", "M2TS", "MKV", "ARI", "MPG", "M4V", "3GP", "INSV", "LRV"]);
const PHOTO = new Set(["JPG", "JPEG", "NEF", "NRW", "CR2", "CR3", "ARW", "DNG", "RAF", "ORF", "RW2", "HEIC", "HEIF", "PNG", "TIF", "TIFF", "PEF", "SRW", "GPR", "IIQ", "3FR"]);
const AUDIO = new Set(["WAV", "BWF", "MP3", "AAC", "M4A", "FLAC", "AIF", "AIFF", "OGG"]);

export function kindOf(ext: string): Kind {
  return VIDEO.has(ext) ? "video" : PHOTO.has(ext) ? "stills" : AUDIO.has(ext) ? "audio" : "other";
}

/** Which kind the grid shows: the one picked, else footage first (video, then audio,
 *  then stills), else everything. Only the view changes; every file is still copied. */
export function gridKind(files: FileView[]): Kind | "all" {
  const has = (k: Kind) => files.some((f) => kindOf(f.ext) === k);
  if (S.kind === "all" || (S.kind && has(S.kind))) return S.kind;
  return (["video", "audio", "stills"] as Kind[]).find(has) ?? "all";
}

/** `files` with the grid's shown kind first, so their thumbnails come first. */
export function byGridKind(files: FileView[]): FileView[] {
  const k = gridKind(files);
  if (k === "all") return files;
  return [...files.filter((f) => kindOf(f.ext) === k), ...files.filter((f) => kindOf(f.ext) !== k)];
}
