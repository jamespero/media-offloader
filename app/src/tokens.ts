// Subfolder tokens: the "Add subfolders" chips under each destination.
// Resolved in the UI; the engine receives the finished folder path.

import type { IconName } from "./icons";

export type TokenType = "import" | "source" | "shot" | "now" | "day" | "reel" | "project" | "text";

export interface Token {
  t: TokenType;
  /** Date format for `now` / `shot`, day format for `day`. */
  fmt?: string;
  /** Text for `text` / `project`, reel letter for `reel`. */
  v?: string;
}

export interface TokenContext {
  sourceLabel: string;
  /** Earliest file date on the card, or null if unknown. */
  shotDate: Date | null;
  importNo: number;
  reelNo: (letter: string) => number;
  /** Shoot day number, set by hand (it never counts up on its own). */
  shootDay: number;
  now: Date;
}

export const DATE_FMTS = [
  "Month DD, YYYY",
  "YY-MM-DD",
  "YYYY-MM-DD",
  "YYYY",
  "YY",
  "Month",
  "MM",
  "DD",
  "Day",
  "Time",
  "Hour",
  "Minute",
  "Second",
  "ISO Datetime",
];

export const TOKEN_TYPES: ({ t: TokenType; label: string; icon: IconName; isNew?: boolean } | { sep: true })[] = [
  { t: "import", label: "Import #", icon: "hash" },
  { t: "source", label: "Source Name", icon: "disk" },
  { t: "shot", label: "Date (Shot/Captured)", icon: "film" },
  { t: "now", label: "Date (Now)", icon: "cal" },
  { sep: true },
  { t: "day", label: "Shoot Day #", icon: "cal", isNew: true },
  { t: "reel", label: "Reel / Roll #", icon: "tag", isNew: true },
  { t: "project", label: "Project Name", icon: "layers", isNew: true },
  { sep: true },
  { t: "text", label: "Custom Text", icon: "type" },
];

export function tokenInfo(t: TokenType) {
  return TOKEN_TYPES.find((x): x is { t: TokenType; label: string; icon: IconName } => "t" in x && x.t === t)!;
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const p2 = (n: number) => String(n).padStart(2, "0");

export function fmtDate(d: Date, f: string): string {
  const Y = d.getFullYear(), M = d.getMonth(), D = d.getDate(), h = d.getHours(), m = d.getMinutes(), s = d.getSeconds();
  const out: Record<string, string> = {
    "Month DD, YYYY": `${MONTHS[M]} ${p2(D)}, ${Y}`,
    "YY-MM-DD": `${String(Y).slice(2)}-${p2(M + 1)}-${p2(D)}`,
    "YYYY-MM-DD": `${Y}-${p2(M + 1)}-${p2(D)}`,
    YYYY: `${Y}`,
    YY: String(Y).slice(2),
    Month: MONTHS[M],
    MM: p2(M + 1),
    DD: p2(D),
    Day: DAYS[d.getDay()],
    Time: `${p2(h)}-${p2(m)}-${p2(s)}`,
    Hour: p2(h),
    Minute: p2(m),
    Second: p2(s),
    "ISO Datetime": `${Y}${p2(M + 1)}${p2(D)}T${p2(h)}${p2(m)}${p2(s)}`,
  };
  return out[f] ?? out["YY-MM-DD"];
}

/** Shoot-day folder formats. The ones with a date use the shot date. */
export const DAY_FMTS = ["Day01", "Day_01", "D01", "SD01", "Day001", "Day01_YYYY-MM-DD", "Day01_YYYYMMDD"];

export function fmtDay(n: number, f: string, date: Date): string {
  const d2 = String(n).padStart(2, "0");
  switch (f) {
    case "Day_01":
      return `Day_${d2}`;
    case "D01":
      return `D${d2}`;
    case "SD01":
      return `SD${d2}`;
    case "Day001":
      return `Day${String(n).padStart(3, "0")}`;
    case "Day01_YYYY-MM-DD":
      return `Day${d2}_${fmtDate(date, "YYYY-MM-DD")}`;
    case "Day01_YYYYMMDD":
      return `Day${d2}_${fmtDate(date, "YYYY-MM-DD").replaceAll("-", "")}`;
    default:
      return `Day${d2}`;
  }
}

/** Characters no destination file system accepts in a folder name. */
export function cleanName(s: string): string {
  return s.replace(/[\\/:*?"<>|\u0000-\u001f]/g, "_").replace(/[. ]+$/, "").trim();
}

export function tokenText(tk: Token, ctx: TokenContext): string {
  switch (tk.t) {
    case "now":
      return fmtDate(ctx.now, tk.fmt ?? "YY-MM-DD");
    case "shot":
      return fmtDate(ctx.shotDate ?? ctx.now, tk.fmt ?? "YY-MM-DD");
    case "source":
      return cleanName(ctx.sourceLabel) || "Source";
    case "day":
      return fmtDay(ctx.shootDay, tk.fmt ?? "Day01", ctx.shotDate ?? ctx.now);
    case "import":
      return "Import " + String(ctx.importNo).padStart(3, "0");
    case "reel": {
      const letter = tk.v || "A";
      return letter + String(ctx.reelNo(letter)).padStart(3, "0");
    }
    case "project":
      return cleanName(tk.v || "Project") || "Project";
    case "text":
      return cleanName(tk.v || "text") || "text";
  }
}

/** The folder the copy goes to: `base` plus one subfolder per token. */
export function resolveRoot(base: string, tokens: Token[], ctx: TokenContext, sep: string): string {
  const trimmed = base.replace(/[\\/]+$/, "");
  const parts = tokens.map((t) => tokenText(t, ctx)).filter(Boolean);
  return [trimmed || sep, ...parts].join(sep);
}

export function hasFormat(t: TokenType) {
  return t === "now" || t === "shot" || t === "day";
}
