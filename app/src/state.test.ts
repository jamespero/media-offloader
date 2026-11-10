import { beforeEach, expect, test, vi } from "vitest";

const KEY = "media-offloader.settings.v1";
const saved = () => JSON.parse(localStorage.getItem(KEY) ?? "null");

let flushSync: () => void;

/** The state module as the app first loads it, with whatever is saved now. */
async function load() {
  vi.resetModules();
  const { S } = await import("./state.svelte.ts");
  // The Svelte that this fresh copy of the module uses, to run its pending effects.
  ({ flushSync } = await import("svelte"));
  flushSync();
  return S;
}

beforeEach(() => localStorage.clear());

test("nothing is written at start-up, so untouched settings follow the app's defaults", async () => {
  await load();
  expect(localStorage.getItem(KEY)).toBeNull();
});

test("any change to the settings is saved, however deep", async () => {
  const S = await load();
  S.settings.sound = false;
  flushSync();
  expect(saved().sound).toBe(false);

  S.settings.tokens.push({ t: "reel", v: "B" });
  S.settings.reels.B = 4;
  flushSync();
  expect(saved().tokens.at(-1)).toEqual({ t: "reel", v: "B" });
  expect(saved().reels).toEqual({ B: 4 });
});

test("saved settings come back on the next start, over the defaults", async () => {
  localStorage.setItem(KEY, JSON.stringify({ sound: false, importNo: 7 }));
  const S = await load();
  expect(S.settings.sound).toBe(false);
  expect(S.settings.importNo).toBe(7);
  expect(S.settings.mhl).toBe(true); // not saved, so the default
});

test("a change made before the saving effect first runs is still saved", async () => {
  vi.resetModules();
  const { S } = await import("./state.svelte.ts");
  S.settings.notify = false; // before any flush
  ({ flushSync } = await import("svelte"));
  flushSync();
  expect(saved().notify).toBe(false);
});
