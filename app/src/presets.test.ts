import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { activePreset, applyPreset, deletePreset, saveCurrentAs } from "./presets";
import { S } from "./state.svelte.ts";

beforeEach(() => {
  vi.useFakeTimers();
  mockIPC(() => null);
  S.settings.presets = [];
  S.settings.tokens = [{ t: "source" }];
  S.phase = "ready";
});
afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
});

test("saving trims the name, refuses an empty one, and matches the current subfolders", () => {
  expect(saveCurrentAs("   ")).toBe(false);
  expect(saveCurrentAs("  A cam ")).toBe(true);
  expect(S.settings.presets).toEqual([{ id: expect.any(String), name: "A cam", tokens: [{ t: "source" }] }]);
  expect(activePreset()?.name).toBe("A cam");
});

test("saving under an existing name (any case) updates that preset and keeps its id", () => {
  saveCurrentAs("A cam");
  const id = S.settings.presets[0].id;
  S.settings.tokens = [{ t: "now", fmt: "YY-MM-DD" }];
  expect(activePreset()).toBeUndefined();
  saveCurrentAs("a CAM");
  expect(S.settings.presets).toEqual([{ id, name: "a CAM", tokens: [{ t: "now", fmt: "YY-MM-DD" }] }]);
});

test("applying copies the preset's subfolders, but not during an offload", () => {
  saveCurrentAs("A cam");
  const id = S.settings.presets[0].id;
  S.settings.tokens = [];
  applyPreset(id);
  expect(S.settings.tokens).toEqual([{ t: "source" }]);

  S.settings.tokens = [];
  S.phase = "copying";
  applyPreset(id);
  expect(S.settings.tokens).toEqual([]);

  deletePreset(id);
  expect(S.settings.presets).toEqual([]);
});
