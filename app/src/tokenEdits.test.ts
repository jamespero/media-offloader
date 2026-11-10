import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { S } from "./state.svelte.ts";
import { addToken, moveToken, removeToken, setReel, setTokenFormat, stepImportNo } from "./tokenEdits";

beforeEach(() => {
  vi.useFakeTimers(); // each edit schedules a re-prepare; these tests don't need it to run
  mockIPC(() => null);
  S.settings.tokens = [];
  S.settings.importNo = 1;
});
afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});

const saved = () => JSON.parse(localStorage.getItem("media-offloader.settings.v1")!);

test("each kind of subfolder starts with its usual value", () => {
  for (const t of ["now", "shot", "day", "text", "reel", "project", "source", "import"] as const) addToken(t);
  expect(S.settings.tokens).toEqual([
    { t: "now", fmt: "YY-MM-DD" },
    { t: "shot", fmt: "YY-MM-DD" },
    { t: "day", fmt: "Day01" },
    { t: "text", v: "text" },
    { t: "reel", v: "A" },
    { t: "project", v: "Project" },
    { t: "source" },
    { t: "import" },
  ]);
  expect(saved().tokens).toHaveLength(8);
});

test("format, reel letter, order and removal", () => {
  addToken("now");
  addToken("reel");
  addToken("source");
  setTokenFormat(0, "YYYYMMDD");
  setReel(1, "C");
  moveToken(2, -1);
  expect(S.settings.tokens).toEqual([{ t: "now", fmt: "YYYYMMDD" }, { t: "source" }, { t: "reel", v: "C" }]);
  moveToken(0, 1);
  removeToken(2);
  expect(S.settings.tokens).toEqual([{ t: "source" }, { t: "now", fmt: "YYYYMMDD" }]);
  expect(saved().tokens).toEqual(S.settings.tokens);
});

test("the Import # counter never goes below 1", () => {
  stepImportNo(1);
  stepImportNo(1);
  expect(S.settings.importNo).toBe(3);
  for (let i = 0; i < 5; i++) stepImportNo(-1);
  expect(S.settings.importNo).toBe(1);
});
