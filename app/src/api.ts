// Typed bridge to the Rust side. Shapes mirror the serde output of offloader-core.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface VolumeEntry {
  path: string;
  label: string;
  removable: boolean;
  fsType: string;
  total: number;
  available: number;
}

export interface PlannedFile {
  srcRel: string;
  destRel: string;
  size: number;
  /** ms since epoch: the card's modification time, roughly when it was shot. */
  modified: number | null;
}

export interface SourceInfo {
  root: string;
  label: string;
  kind: "volume" | "folder";
}

export interface Plan {
  source: SourceInfo;
  files: PlannedFile[];
  skipped: { rel: string; reason: "systemFile" | "symlink" | "notARegularFile" | "unreadableSystemFolder" }[];
  problems: { rel: string; message: string }[];
  renamed: { from: string; to: string; reason: "illegalName" | "caseCollision" }[];
  totalBytes: number;
}

export interface Destination {
  role: "copy" | "backup";
  requested: string;
  /** Where files go: `requested`, or `requested/copy-conflicted[-N]`. */
  root: string;
  conflict: number | null;
  fsType: string;
  available: number;
  required: number;
}

export interface Prepared {
  plan: Plan;
  destinations: Destination[];
  /** Journal of this offload (for resume and history). */
  journal: string | null;
}

export interface Hashes {
  xxh64: string;
  md5?: string;
}

export type FileFailure =
  | { kind: "sourceRead" | "write" | "verifyRead"; message: string }
  | { kind: "sourceChanged"; when: string }
  | { kind: "mismatch" | "copyChanged"; expected: Hashes; actual: Hashes }
  | { kind: "alreadyExists"; path: string };

export type FileStatus = { state: "pending" } | { state: "verified" } | { state: "failed"; failure: FileFailure };

export type Pass = "copy" | "backup" | "fanOut";
export type Stage = "copy" | "verify";

export type Outcome =
  | { kind: "verified" }
  | { kind: "failed"; files: number }
  | { kind: "cancelled" }
  | { kind: "aborted"; reason: string };

export type EngineEvent =
  | { type: "passStarted"; pass: Pass; targets: number[]; files: number; bytes: number }
  | { type: "fileStarted"; pass: Pass; file: number }
  | { type: "progress"; pass: Pass; file: number; stage: Stage; fileBytes: number; done: number; total: number }
  | { type: "fileFinished"; pass: Pass; file: number; hashes: Hashes | null; results: [number, FileStatus][] }
  | { type: "passFinished"; pass: Pass; verified: number; failed: number }
  | { type: "sourceReleased" }
  | { type: "warning"; message: string }
  | { type: "finished"; outcome: Outcome };

export interface JobReport {
  outcome: Outcome;
  sourceReleased: boolean;
  started: number | null;
  finished: number | null;
  files: { hashes: Hashes | null; dests: FileStatus[] }[];
  logPath: string | null;
  sidecars: string[];
  warnings: string[];
}

export interface OffloadArgs {
  source: string;
  copyTo: string;
  backupTo: string | null;
  md5: boolean;
  fanOut: boolean;
  includeSystemFiles: boolean;
  mhl: boolean;
  md5List: boolean;
  keepAwake: boolean;
}

export const api = {
  listSources: () => invoke<VolumeEntry[]>("list_sources"),
  previewSource: (path: string, includeSystemFiles: boolean) => invoke<Plan>("preview_source", { path, includeSystemFiles }),
  prepare: (args: OffloadArgs) => invoke<Prepared>("prepare", { args }),
  start: () => invoke<void>("start"),
  cancel: () => invoke<void>("cancel"),
  isRunning: () => invoke<boolean>("is_running"),
  readText: (path: string) => invoke<string>("read_text", { path }),
  eject: (path: string) => invoke<void>("eject", { path }),
  logDir: () => invoke<string | null>("log_dir"),
  reveal: (path: string) => invoke<void>("reveal", { path }),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
  onEngine: (f: (e: EngineEvent) => void): Promise<UnlistenFn> => listen<EngineEvent>("engine", (e) => f(e.payload)),
  onReport: (f: (r: JobReport) => void): Promise<UnlistenFn> => listen<JobReport>("engine-report", (e) => f(e.payload)),
};

export function failureText(f: FileFailure): string {
  switch (f.kind) {
    case "sourceRead":
      return `Couldn't read the card: ${f.message}`;
    case "write":
      return `Couldn't write: ${f.message}`;
    case "verifyRead":
      return `Couldn't read back to verify: ${f.message}`;
    case "sourceChanged":
      return `The file changed ${f.when}`;
    case "mismatch":
      return "Checksum mismatch — not verified";
    case "copyChanged":
      return "The verified copy changed before backup";
    case "alreadyExists":
      return "A file with that name appeared; not overwritten";
  }
}

export type HashFormat = "c4" | "md5" | "sha1" | "xxh128" | "xxh3" | "xxh64";

export type CheckStatus =
  | { state: "pending" }
  | { state: "verified" }
  | { state: "mismatch"; format: HashFormat; expected: string; actual: string }
  | { state: "sizeDiffers"; expected: number; actual: number }
  | { state: "missing" }
  | { state: "readError"; message: string };

export type ManifestSource = { kind: "mhl"; generations: number } | { kind: "md5List"; file: string };
export type ReverifyOutcome = { kind: "verified" } | { kind: "failed"; files: number } | { kind: "cancelled" };

export type ReverifyEvent =
  | { type: "started"; source: ManifestSource; files: number; bytes: number }
  | { type: "fileStarted"; file: number; path: string }
  | { type: "progress"; file: number; fileBytes: number; done: number; total: number }
  | { type: "fileFinished"; file: number; status: CheckStatus }
  | { type: "finished"; outcome: ReverifyOutcome };

export interface ReverifyReport {
  root: string;
  source: ManifestSource;
  files: { path: string; size: number | null; formats: HashFormat[]; status: CheckStatus }[];
  notListed: string[];
  manifestProblems: string[];
  outcome: ReverifyOutcome;
  logPath: string | null;
}

export const reverifyApi = {
  start: (path: string) => invoke<void>("reverify_folder", { path }),
  cancel: () => invoke<void>("cancel_reverify"),
  onEvent: (f: (e: ReverifyEvent) => void) => listen<ReverifyEvent>("reverify", (e) => f(e.payload)),
  onReport: (f: (r: ReverifyReport) => void) => listen<ReverifyReport>("reverify-report", (e) => f(e.payload)),
  onError: (f: (msg: string) => void) => listen<string>("reverify-error", (e) => f(e.payload)),
};

/** A past offload, from its journal. */
export interface HistoryItem {
  path: string;
  started: number;
  finished: number | null;
  label: string;
  source: string;
  sourceKind: "volume" | "folder";
  sourcePresent: boolean;
  roots: string[];
  files: number;
  bytes: number;
  /** Per destination. */
  verified: number[];
  failed: number[];
  released: boolean;
  /** null: the app stopped before the offload finished. */
  outcome: Outcome | null;
  resumable: boolean;
  log: string | null;
  sidecars: string[];
  runs: number;
}

export interface HistoryDetail {
  item: HistoryItem;
  files: { src: string; dest: string; size: number; modified: number | null }[];
  verified: string[][];
  failed: Record<string, string>[];
}

export const historyApi = {
  list: (limit: number) => invoke<HistoryItem[]>("history_list", { limit }),
  detail: (path: string) => invoke<HistoryDetail>("history_detail", { path }),
  forget: (path: string) => invoke<void>("forget_history", { path }),
  prepareResume: (journalPath: string) => invoke<Prepared>("prepare_resume", { journalPath }),
};

export const thumbsApi = {
  request: (generation: number, items: { key: string; path: string }[]) => invoke<void>("thumbs_request", { generation, items }),
  onThumb: (f: (t: { generation: number; key: string; url: string | null }) => void) =>
    listen<{ generation: number; key: string; url: string | null }>("thumb", (e) => f(e.payload)),
};
