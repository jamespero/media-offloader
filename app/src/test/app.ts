// Starts the whole app the way the browser preview does: `main.ts`, which (outside Tauri, in
// development) loads the preview's simulated backend. Each start gets fresh modules and
// settings, fake timers so the simulated offload runs instantly, and a record of every call
// that reaches the backend.

import { fireEvent, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { unmount } from "svelte";
import { vi } from "vitest";

export interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

/** Calls that happen on their own (polling, events) rather than because of what a test did. */
const BACKGROUND = /^(list_sources|is_running)$|^plugin:(event|updater|app)/;

let stopCurrent: (() => void) | null = null;

/** `search` is the preview's URL options, e.g. "?fail" for two files failing verification. */
export async function startApp(search = "") {
  stopApp();
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.resetModules();
  localStorage.clear();
  history.replaceState(null, "", "/" + search);
  const target = document.createElement("div");
  target.id = "app";
  document.body.replaceChildren(target);

  const main = await import("../main");
  const { S } = await import("../state.svelte.ts");

  const calls: Call[] = [];
  const internals = (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown, opts?: unknown) => unknown } })
    .__TAURI_INTERNALS__;
  const invoke = internals.invoke;
  internals.invoke = (cmd, args, opts) => {
    if (!BACKGROUND.test(cmd)) calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    return invoke(cmd, args, opts);
  };

  stopCurrent = () => unmount(main.app);
  await settle();

  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
  const panel = (id: "pSrc" | "pCopy" | "pBak") => within(document.getElementById(id)!);
  const menu = () => within(document.querySelector<HTMLElement>(".menu")!);
  const dialog = () => within(document.getElementById("scrim")!);
  /** Every recorded call to `cmd`, and clears the record. */
  const take = (cmd?: string) => {
    const all = calls.splice(0);
    return cmd ? all.filter((c) => c.cmd === cmd) : all;
  };

  return { S, calls, take, user, panel, menu, dialog, screen, fireEvent };
}

export function stopApp() {
  stopCurrent?.();
  stopCurrent = null;
  vi.clearAllTimers();
  vi.useRealTimers();
}

/** Lets timers and promises run, e.g. the simulated backend's short delays. */
export async function settle(ms = 1000) {
  await vi.advanceTimersByTimeAsync(ms);
}

/** Runs time forward until `done()`, e.g. through a whole simulated offload. */
export async function runUntil(done: () => boolean, ms = 120_000) {
  for (let t = 0; t < ms && !done(); t += 250) await vi.advanceTimersByTimeAsync(250);
  if (!done()) throw new Error("timed out");
  await settle(500);
}
