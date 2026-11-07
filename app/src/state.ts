// App state. One offload at a time.

import type { Destination, HistoryDetail, HistoryItem, JobReport, Plan, VolumeEntry } from "./api";
import type { Preset } from "./presets";
import type { Token, TokenContext } from "./tokens";

export type Phase = "waiting" | "ready" | "copying" | "backing" | "done" | "error";
export type ThumbState = "pending" | "copying" | "copied" | "backing" | "backed" | "failed";

export interface FileView {
  srcRel: string;
  destRel: string;
  name: string;
  ext: string;
  size: number;
  modified: number | null;
  state: ThumbState;
  /** 0–100 progress within the current stage. */
  p: number;
  verify: boolean;
  failure?: string;
}

export interface Source {
  path: string;
  label: string;
  kind: "card" | "drive" | "folder";
  media: string;
}

/** Persisted between launches. */
export interface Settings {
  md5: boolean;
  fanOut: boolean;
  includeSystemFiles: boolean;
  mhl: boolean;
  md5List: boolean;
  autoSelect: boolean;
  ejectWhenVerified: boolean;
  keepAwake: boolean;
  sound: boolean;
  notify: boolean;
  importNo: number;
  reels: Record<string, number>;
  shootDay: number;
  tokens: Token[];
  presets: Preset[];
  copyDest: string | null;
  bakDest: string | null;
  recentCopy: string[];
  recentBak: string[];
}

export const isWin = navigator.userAgent.includes("Windows");
export const sep = isWin ? "\\" : "/";
export const fileManager = isWin ? "Explorer" : "Finder";

const DEFAULTS: Settings = {
  md5: false,
  fanOut: false,
  includeSystemFiles: false,
  mhl: true,
  md5List: false,
  autoSelect: true,
  ejectWhenVerified: false,
  keepAwake: true,
  sound: true,
  notify: true,
  importNo: 1,
  reels: {},
  shootDay: 1,
  tokens: [{ t: "now", fmt: "YY-MM-DD" }, { t: "source" }],
  presets: [],
  copyDest: null,
  bakDest: null,
  recentCopy: [],
  recentBak: [],
};

const KEY = "media-offloader.settings.v1";

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const s: Settings = { ...DEFAULTS, ...JSON.parse(raw) };
      // Presets used to carry destinations too; now they're just the pattern.
      s.presets = s.presets.map(({ id, name, tokens }) => ({ id, name, tokens }));
      return s;
    }
  } catch {
    /* fall back to defaults */
  }
  return structuredClone(DEFAULTS);
}

export function saveSettings() {
  try {
    localStorage.setItem(KEY, JSON.stringify(S.settings));
  } catch {
    /* storage unavailable: settings last for this session only */
  }
}

export interface Live {
  name: string;
  verify: boolean;
}

export const S = {
  settings: load(),
  volumes: [] as VolumeEntry[],
  src: null as Source | null,
  plan: null as Plan | null,
  files: [] as FileView[],
  phase: "waiting" as Phase,
  scanning: false,
  /** Result of the latest `prepare`: resolved folders (copy-conflicted), space. */
  dests: null as Destination[] | null,
  prepError: null as string | null,
  copyP: 0,
  bakP: 0,
  cur: null as Live | null,
  curB: null as Live | null,
  /** Effective offload speed in bytes/s, and seconds left. */
  rate: 0,
  eta: null as number | null,
  released: false,
  /** The source was ejected (or unplugged) after its copy verified. */
  ejected: false,
  /** Files that failed on the copy drive / on the backup drive in the last run. */
  failCopy: 0,
  failBak: 0,
  /** Whether the current (or last) run included a backup. */
  runHadBackup: false,
  /** Whether the current (or last) run is fan-out (a resumed run uses its own mode). */
  runFanOut: false,
  abortReason: null as string | null,
  /** After a Stop: how many verified files were kept. */
  stoppedKept: 0,
  report: null as JobReport | null,
  startedAt: null as Date | null,
  finishedAt: null as Date | null,
  /** Past offloads, newest first. */
  history: [] as HistoryItem[],
  /** Which tab is showing: "" for Current, else a journal path. */
  view: "",
  detail: null as HistoryDetail | null,
  /** Files of the history tab being shown. */
  histFiles: [] as FileView[],
  /** Continuing this journal (resume / retry) rather than starting fresh. */
  resumeJournal: null as string | null,
  /** Kind of file the grid shows; null picks footage first (see `gridKind`). */
  kind: null as "video" | "stills" | "audio" | "other" | "all" | null,
  /** Journal of the offload on the Current tab. */
  currentJournal: null as string | null,
};

export function tokenContext(now = new Date()): TokenContext {
  const shot = S.files.reduce<number | null>((min, f) => (f.modified != null && (min == null || f.modified < min) ? f.modified : min), null);
  return {
    sourceLabel: S.src?.label ?? "Source",
    shotDate: shot != null ? new Date(shot) : null,
    importNo: S.settings.importNo,
    reelNo: (l) => S.settings.reels[l] ?? 1,
    shootDay: S.settings.shootDay,
    now,
  };
}

export function totalBytes() {
  return S.files.reduce((a, f) => a + f.size, 0);
}

export function filesFromPlan(plan: Plan): FileView[] {
  return plan.files.map((f) => fileView(f.destRel, f.srcRel, f.size, f.modified));
}

export const running = () => S.phase === "copying" || S.phase === "backing";

/** The files on screen: the Current offload's, or the history tab's. */
export const shownFiles = () => (S.view ? S.histFiles : S.files);

export function fileView(destRel: string, srcRel: string, size: number, modified: number | null): FileView {
  const name = destRel.split("/").pop() ?? destRel;
  const dot = name.lastIndexOf(".");
  return { srcRel, destRel, name, ext: dot > 0 ? name.slice(dot + 1).toUpperCase() : "", size, modified, state: "pending", p: 0, verify: false };
}

/** Whether the Current offload has a backup: the chosen one, or a resumed offload's. */
export const hasBackup = () => (S.resumeJournal ? (S.dests?.length ?? 0) > 1 : !!S.settings.bakDest);
