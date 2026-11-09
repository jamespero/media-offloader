// App state, reactive so Svelte components update themselves. One offload at a time.

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
  /** Label of the check-mark flash playing on this tile, if any. */
  flash?: string;
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
  autoUpdate: boolean;
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
  autoUpdate: false,
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

class AppState {
  settings = $state(load());
  volumes = $state.raw<VolumeEntry[]>([]);
  src = $state.raw<Source | null>(null);
  plan = $state.raw<Plan | null>(null);
  files = $state<FileView[]>([]);
  phase = $state<Phase>("waiting");
  scanning = $state(false);
  /** Result of the latest `prepare`: resolved folders (copy-conflicted), space. */
  dests = $state.raw<Destination[] | null>(null);
  prepError = $state<string | null>(null);
  copyP = $state(0);
  bakP = $state(0);
  cur = $state.raw<Live | null>(null);
  curB = $state.raw<Live | null>(null);
  /** Effective offload speed in bytes/s, and seconds left. */
  rate = $state(0);
  eta = $state<number | null>(null);
  released = $state(false);
  /** The source was ejected (or unplugged) after its copy verified. */
  ejected = $state(false);
  /** Files that failed on the copy drive / on the backup drive in the last run. */
  failCopy = $state(0);
  failBak = $state(0);
  /** Whether the current (or last) run included a backup. */
  runHadBackup = $state(false);
  /** Whether the current (or last) run is fan-out (a resumed run uses its own mode). */
  runFanOut = $state(false);
  abortReason = $state<string | null>(null);
  /** After a Stop: how many verified files were kept. */
  stoppedKept = $state(0);
  report = $state.raw<JobReport | null>(null);
  startedAt = $state.raw<Date | null>(null);
  finishedAt = $state.raw<Date | null>(null);
  /** Past offloads, newest first. */
  history = $state.raw<HistoryItem[]>([]);
  /** Which tab is showing: "" for Current, else a journal path. */
  view = $state("");
  detail = $state.raw<HistoryDetail | null>(null);
  /** Files of the history tab being shown. */
  histFiles = $state<FileView[]>([]);
  /** Continuing this journal (resume / retry) rather than starting fresh. */
  resumeJournal = $state<string | null>(null);
  /** The earlier offload that `resumeJournal` continues, for marking its finished files. */
  resumeDetail = $state.raw<HistoryDetail | null>(null);
  /** Kind of file the grid shows; null picks footage first (see `gridKind`). */
  kind = $state<"video" | "stills" | "audio" | "other" | "all" | null>(null);
  /** Journal of the offload on the Current tab. */
  currentJournal = $state<string | null>(null);
}

export const S = new AppState();

/** A plain copy of reactive data; `structuredClone` can't copy Svelte's state proxies. */
export const plain = <T>(value: T): T => $state.snapshot(value) as T;

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
