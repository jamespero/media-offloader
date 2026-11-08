// Dev-only browser preview: run the real UI in a plain browser with Tauri's IPC mocked
// and a simulated engine, for design work and screenshots. main.ts loads this only in
// `vite dev` and only outside Tauri, so it never ships in the app.
//
//   http://localhost:1420/            normal run
//   http://localhost:1420/?fail       two files fail verification
//   http://localhost:1420/?slow       slower simulation
//   http://localhost:1420/?update     an update is available (offered about 7 s after load)

import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { version } from "../package.json";
import type { Destination, EngineEvent, HistoryDetail, HistoryItem, JobReport, OffloadArgs, Plan, PlannedFile, VolumeEntry } from "./api";

const GB = 1e9;
const MB = 1e6;
const params = new URLSearchParams(location.search);
const isWin = navigator.userAgent.includes("Windows");
const sep = isWin ? "\\" : "/";

function rng(seed: number) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const base = Date.parse("2026-10-05T08:12:00");

function card(kind: "zr" | "ninja" | "f6"): PlannedFile[] {
  const r = rng(kind === "zr" ? 7 : kind === "ninja" ? 23 : 11);
  const out: PlannedFile[] = [];
  const add = (rel: string, size: number, i: number) =>
    out.push({ srcRel: rel, destRel: rel, size: Math.round(size), modified: base + i * (240 + r() * 600) * 1000 });
  if (kind === "zr") {
    for (let i = 0; i < 36; i++) {
      const x = r();
      const [ext, size] = x < 0.45 ? ["NEV", (1.6 + r() * 3.6) * GB] : x < 0.78 ? ["MOV", (0.4 + r() * 1.4) * GB] : ["NEF", (44 + r() * 14) * MB];
      add(`DCIM/100NCZ_R/DSC_${String(i + 1).padStart(4, "0")}.${ext}`, size as number, i);
    }
    // Like a real ZR card: JPEG stills, and the camera's own LUTs and settings.
    for (let i = 0; i < 12; i++) add(`DCIM/100NCZ_R/DSC_${String(40 + i).padStart(4, "0")}.JPG`, (8 + r() * 6) * MB, 36 + i);
    for (let i = 1; i <= 6; i++) add(`NIKON/LUTS/LUT_${String(i).padStart(2, "0")}.cube`, 1.2 * MB, 48);
    add("NIKON/ZR/ZR__0111.bin", 0.1 * MB, 48);
  } else if (kind === "ninja") {
    for (let i = 1; i <= 14; i++) add(`NINJA2_S001_S001_T${String(i).padStart(3, "0")}.MOV`, (0.8 + r() * 3) * GB, i);
  } else {
    for (let i = 1; i <= 9; i++) add(`FOLDER01/261005_${String(i).padStart(3, "0")}.WAV`, (180 + r() * 700) * MB, i);
  }
  return out;
}

const root = (label: string, letter: string) => (isWin ? `${letter}:\\` : `/Volumes/${label}`);
const VOLUMES: (VolumeEntry & { kind: "zr" | "ninja" | "f6" })[] = [
  { kind: "zr", path: root("ZR_CARD_A", "E"), label: "ZR_CARD_A", removable: true, fsType: "exFAT", total: 256e9, available: 12e9 },
  { kind: "ninja", path: root("NINJA2", "G"), label: "NINJA2", removable: false, fsType: "exFAT", total: 1e12, available: 400e9 },
  { kind: "f6", path: root("F6_SD", "H"), label: "F6_SD", removable: true, fsType: "FAT32", total: 64e9, available: 50e9 },
];

function planFor(path: string): Plan {
  const v = VOLUMES.find((x) => x.path === path);
  const files = v ? card(v.kind) : card("zr").slice(0, 10).map((f, i) => ({ ...f, srcRel: `select_${i + 1}.MOV`, destRel: `select_${i + 1}.MOV` }));
  return {
    source: { root: path, label: v?.label ?? path.split(/[\\/]/).filter(Boolean).pop() ?? "Folder", kind: v ? "volume" : "folder" },
    files,
    skipped: v?.kind === "zr" ? [{ rel: ".Spotlight-V100", reason: "systemFile" }] : [],
    problems: [],
    renamed: [],
    totalBytes: files.reduce((a, f) => a + f.size, 0),
  };
}

let prepared: { args: OffloadArgs; plan: Plan } | null = null;
let cancelled = false;
let running = false;

function dest(role: "copy" | "backup", requested: string, plan: Plan, conflict: boolean): Destination {
  return {
    role,
    requested,
    root: conflict ? requested + sep + "copy-conflicted" : requested,
    conflict: conflict ? 0 : null,
    fsType: "exFAT",
    available: 2e12,
    required: plan.totalBytes,
  };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const send = (e: EngineEvent) => emit("engine", e);

async function simulate() {
  const { args, plan } = prepared!;
  running = true;
  cancelled = false;
  const files = plan.files;
  const total = plan.totalBytes;
  const failSet = params.has("fail") ? new Set([7, 19]) : new Set<number>();
  const secs = params.has("slow") ? 40 : 12;
  const speed = (total * 2) / secs; // work units per second
  const hasBackup = !!args.backupTo;
  const targets = hasBackup && args.fanOut ? [0, 1] : [0];
  const report: JobReport = {
    outcome: { kind: "verified" },
    sourceReleased: false,
    started: Date.now(),
    finished: null,
    files: files.map(() => ({ hashes: null, dests: targets.concat(hasBackup && !args.fanOut ? [1] : []).map(() => ({ state: "pending" as const })) })),
    logPath: "preview.log",
    sidecars: [],
    warnings: [],
  };

  const pass = async (name: "copy" | "backup" | "fanOut", tg: number[]) => {
    send({ type: "passStarted", pass: name, targets: tg, files: files.length, bytes: total });
    let done = 0;
    for (let i = 0; i < files.length; i++) {
      const f = files[i];
      send({ type: "fileStarted", pass: name, file: i });
      for (const stage of ["copy", "verify"] as const) {
        let b = 0;
        while (b < f.size) {
          if (cancelled) return false;
          await sleep(33);
          b = Math.min(f.size, b + speed * 0.033 * (stage === "verify" ? 1.6 : 1));
          send({ type: "progress", pass: name, file: i, stage, fileBytes: b, done: done + (stage === "verify" ? f.size : 0) + b, total: total * 2 });
        }
      }
      done += f.size * 2;
      const bad = name !== "backup" && failSet.has(i);
      const status = bad
        ? { state: "failed" as const, failure: { kind: "mismatch" as const, expected: { xxh64: "3f2a19bc0e7d5a11" }, actual: { xxh64: "9c41e07a52d3b8f0" } } }
        : { state: "verified" as const };
      tg.forEach((t) => (report.files[i].dests[t] = status));
      send({ type: "fileFinished", pass: name, file: i, hashes: { xxh64: "3f2a19bc0e7d5a11" }, results: tg.map((t) => [t, status]) });
    }
    send({ type: "passFinished", pass: name, verified: files.length - failSet.size, failed: failSet.size });
    return true;
  };

  let ok = await pass(args.fanOut ? "fanOut" : "copy", targets);
  if (ok && !failSet.size) {
    report.sourceReleased = true;
    send({ type: "sourceReleased" });
    if (hasBackup && !args.fanOut) ok = await pass("backup", [1]);
  }
  report.outcome = cancelled ? { kind: "cancelled" } : failSet.size ? { kind: "failed", files: failSet.size } : { kind: "verified" };
  report.finished = Date.now();
  send({ type: "finished", outcome: report.outcome });
  await emit("engine-report", report);
  running = false;
}

const LOG = `Media Offloader ${version} — transfer log (browser preview)
Started      2026-10-05 14:12:15 -04:00
Source       ZR_CARD_A  (E:\\)
Copy         D:\\Jobs\\2026-Fall\\26-10-05\\ZR_CARD_A
Mode         sequential (copy, then back up from the verified copy) · xxHash64 · verify reads bypass the OS cache
Files        36 · 98.2 GB

== copy → D:\\Jobs\\2026-Fall\\26-10-05\\ZR_CARD_A ==
OK    copy   bc0d57a116140a9d  DCIM/100NCZ_R/DSC_0001.NEV  4.2 GB
OK    copy   c2f876ec461d0495  DCIM/100NCZ_R/DSC_0002.NEF  52 MB
FAIL  copy   DCIM/100NCZ_R/DSC_0008.MOV  1.2 GB  — checksum mismatch: source xxh64 3f2a19bc0e7d5a11, destination xxh64 9c41e07a52d3b8f0

Finished     2026-10-05 14:20:27 -04:00  (8m 12s)
Result       All files verified.`;

mockWindows("main");
mockIPC(
  async (cmd, payload) => {
    const p = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "list_sources":
        return VOLUMES.map(({ kind: _kind, ...v }) => v);
      case "preview_source":
        await sleep(300);
        return planFor(p.path as string);
      case "prepare": {
        const args = p.args as OffloadArgs;
        const plan = planFor(args.source);
        prepared = { args, plan };
        const again = plan.source.label === "NINJA2";
        return {
          plan,
          destinations: [dest("copy", args.copyTo, plan, again), ...(args.backupTo ? [dest("backup", args.backupTo, plan, again)] : [])],
          journal: null,
        };
      }
      case "history_list":
        return HISTORY.filter((h) => !forgotten.has(h.path));
      case "history_detail":
        return historyDetail(p.path as string);
      case "forget_history":
        forgotten.add(p.path as string);
        return null;
      case "prepare_resume": {
        const h = HISTORY.find((x) => x.path === p.journalPath)!;
        const plan = planFor(h.source);
        const args = { source: h.source, copyTo: h.roots[0], backupTo: h.roots[1] ?? null, md5: false, fanOut: false, includeSystemFiles: false, mhl: true, md5List: false, keepAwake: true };
        prepared = { args, plan };
        return { plan, destinations: h.roots.map((r, i) => dest(i ? "backup" : "copy", r, plan, false)), journal: h.path };
      }
      case "start":
        simulate();
        return null;
      case "cancel":
        cancelled = true;
        return null;
      case "is_running":
        return running;
      case "read_text":
        return LOG;
      case "log_dir":
        return isWin ? "C:\\Users\\you\\AppData\\Local\\Media Offloader\\Logs" : "~/Library/Logs/Media Offloader";
      case "reverify_folder":
        simulateReverify(p.path as string);
        return null;
      case "cancel_reverify":
        rvCancelled = true;
        return null;
      case "eject":
        await sleep(400);
        return null;
      case "open_repo":
        window.open("https://github.com/jamespero/media-offloader", "_blank");
        return null;
      case "plugin:app|version":
        return version;
      case "plugin:updater|check":
        return params.has("update")
          ? { rid: 1, currentVersion: version, version: "9.9.9", date: new Date().toISOString(), body: "", rawJson: {} }
          : null;
      case "plugin:updater|download":
        await sleep(2000);
        return null;
      case "plugin:updater|install":
        await sleep(1500);
        return null;
      case "plugin:process|restart":
        location.reload();
        return null;
      case "open_release":
        window.open(`https://github.com/jamespero/media-offloader/releases/tag/v${String(p.version)}`, "_blank");
        return null;
      case "plugin:dialog|open": {
        const title = String((p.options as { title?: string } | undefined)?.title ?? "");
        if (title.startsWith("Back up")) return isWin ? "F:\\Jobs\\2026-Fall" : "/Volumes/SHUTTLE_B/Jobs/2026-Fall";
        if (title.includes("folder to offload")) return isWin ? "C:\\Users\\you\\Desktop\\Day 3 selects" : "/Users/you/Desktop/Day 3 selects";
        return isWin ? "D:\\Jobs\\2026-Fall" : "/Volumes/SHUTTLE_A/Jobs/2026-Fall";
      }
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

let rvCancelled = false;

async function simulateReverify(path: string) {
  rvCancelled = false;
  const files = card("zr").slice(0, 20);
  const total = files.reduce((a, f) => a + f.size, 0);
  const bad = params.has("fail") ? 4 : -1;
  await emit("reverify", { type: "started", source: { kind: "mhl", generations: 2 }, files: files.length, bytes: total });
  let done = 0;
  const results = [];
  for (let i = 0; i < files.length && !rvCancelled; i++) {
    const f = files[i];
    await emit("reverify", { type: "fileStarted", file: i, path: f.destRel });
    for (let b = 0; b < f.size && !rvCancelled; b += total / 120) {
      await sleep(30);
      await emit("reverify", { type: "progress", file: i, fileBytes: b, done: done + b, total });
    }
    done += f.size;
    const status =
      i === bad ? { state: "mismatch", format: "xxh64", expected: "3f2a19bc0e7d5a11", actual: "9c41e07a52d3b8f0" } : { state: "verified" };
    results.push({ path: f.destRel, size: f.size, formats: ["xxh64"], status });
    await emit("reverify", { type: "fileFinished", file: i, status });
  }
  const failed = results.filter((r) => r.status.state !== "verified").length;
  const outcome = rvCancelled ? { kind: "cancelled" } : failed ? { kind: "failed", files: failed } : { kind: "verified" };
  await emit("reverify", { type: "finished", outcome });
  await emit("reverify-report", {
    root: path,
    source: { kind: "mhl", generations: 2 },
    files: results,
    notListed: params.has("fail") ? ["DCIM/100NCZ_R/DSC_9999.JPG"] : [],
    manifestProblems: [],
    outcome,
    logPath: "preview.log",
  });
}

/* ------------------------------------------------------------------ history */

const forgotten = new Set<string>();
const day = (d: string) => Date.parse(d);
const jobs = isWin ? "C:\\Users\\you\\AppData\\Local\\Media Offloader\\Jobs\\" :"~/Library/Application Support/Media Offloader/Jobs/";
const HISTORY: HistoryItem[] = [
  {
    path: jobs + "20261005-081400_ZR_CARD_A.jsonl",
    started: day("2026-10-05T08:14:00"),
    finished: null,
    label: "ZR_CARD_A",
    source: VOLUMES[0].path,
    sourceKind: "volume",
    sourcePresent: true,
    roots: [isWin ? "D:\\Jobs\\2026-Fall\\26-10-05\\ZR_CARD_A" :"/Volumes/SHUTTLE_A/Jobs/2026-Fall/26-10-05/ZR_CARD_A"],
    files: 36,
    bytes: planFor(VOLUMES[0].path).totalBytes,
    verified: [12],
    failed: [0],
    released: false,
    outcome: null,
    resumable: true,
    log: "preview.log",
    sidecars: [],
    runs: 1,
  },
  {
    path: jobs + "20261004-091355_NINJA2.jsonl",
    started: day("2026-10-04T09:13:55"),
    finished: day("2026-10-04T09:41:10"),
    label: "NINJA2",
    source: VOLUMES[1].path,
    sourceKind: "volume",
    sourcePresent: true,
    roots: isWin ? ["D:\\Jobs\\2026-Fall\\26-10-04\\NINJA2", "F:\\Jobs\\2026-Fall\\26-10-04\\NINJA2"] :["/Volumes/SHUTTLE_A/26-10-04/NINJA2", "/Volumes/SHUTTLE_B/26-10-04/NINJA2"],
    files: 14,
    bytes: planFor(VOLUMES[1].path).totalBytes,
    verified: [14, 14],
    failed: [0, 0],
    released: true,
    outcome: { kind: "verified" },
    resumable: false,
    log: "preview.log",
    sidecars: [],
    runs: 1,
  },
  {
    path: jobs + "20261003-174940_F6_SD.jsonl",
    started: day("2026-10-03T17:49:40"),
    finished: day("2026-10-03T17:52:02"),
    label: "F6_SD",
    source: VOLUMES[2].path,
    sourceKind: "volume",
    sourcePresent: false,
    roots: [isWin ? "D:\\Jobs\\2026-Fall\\26-10-03\\F6_SD" :"/Volumes/SHUTTLE_A/26-10-03/F6_SD"],
    files: 9,
    bytes: planFor(VOLUMES[2].path).totalBytes,
    verified: [8],
    failed: [1],
    released: false,
    outcome: { kind: "failed", files: 1 },
    resumable: true,
    log: "preview.log",
    sidecars: [],
    runs: 1,
  },
];

function historyDetail(path: string): HistoryDetail {
  const item = HISTORY.find((h) => h.path === path)!;
  const files = planFor(item.source).files.map((f) => ({ src: f.srcRel, dest: f.destRel, size: f.size, modified: f.modified }));
  const verified = item.roots.map((_, d) => files.slice(0, item.verified[d]).map((f) => f.dest));
  const failed = item.roots.map((_, d) => (item.failed[d] ? { [files[files.length - 1].dest]: "checksum mismatch" } : {}));
  return { item, files, verified, failed };
}
