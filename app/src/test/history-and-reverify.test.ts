// Past offloads in the strip above the grid, resuming one, and re-verifying a folder.

import { within } from "@testing-library/svelte";
import { afterEach, expect, test } from "vitest";
import { runUntil, settle, startApp, stopApp } from "./app";

afterEach(stopApp);

const tabs = () => [...document.querySelectorAll<HTMLButtonElement>(".strip .tab")];
/** A past offload's tab; tabs run oldest first, with Current last. */
const tab = (label: string) => document.querySelector<HTMLButtonElement>(`.strip .tab[title="${label}"]`)!;

test("a past offload opens from its tab, with its log, and Current goes back", async () => {
  const { S, take, user, panel, dialog } = await startApp();
  expect(tabs()).toHaveLength(4); // three past offloads, then Current
  await user.click(tab("NINJA2")); // verified
  await settle();
  expect(take("history_detail")).toHaveLength(1);
  expect(S.detail?.item.label).toBe("NINJA2");
  expect(panel("pSrc").getByText("Copied & verified")).toBeInTheDocument();

  await user.click(panel("pCopy").getByTitle("Open the log for this offload"));
  await settle();
  expect(take("read_text")).toEqual([{ cmd: "read_text", args: { path: S.detail!.item.log } }]);
  expect(dialog().getByText("Transfer log")).toBeInTheDocument();
  await user.keyboard("{Escape}");

  await user.click(tabs().at(-1)!);
  expect(S.view).toBe("");
});

test("the file-type filter shows one kind, and All shows every file", async () => {
  const { S, user, screen, menu } = await startApp();
  await user.click(screen.getByRole("button", { name: /Select a source/ }));
  await user.click(menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await settle();
  const kinds = document.querySelector<HTMLElement>(".kinds")!;
  const video = [...kinds.querySelectorAll("button")].find((b) => b.textContent!.startsWith("Video"))!;
  const all = [...kinds.querySelectorAll("button")].find((b) => b.textContent!.startsWith("All"))!;
  await user.click(all);
  expect(S.kind).toBe("all");
  expect(document.querySelectorAll(".thumb")).toHaveLength(S.files.length);
  await user.click(video);
  expect(S.kind).toBe("video");
  expect(document.querySelectorAll(".thumb").length).toBeLessThan(S.files.length);
});

test("the banner offers to resume the last offload when it didn't finish", async () => {
  const { S, take, user } = await startApp();
  const grid = document.getElementById("gridwrap")!;
  expect(grid).toHaveTextContent("The offload of ZR_CARD_A");
  expect(grid).toHaveTextContent("didn't finish");
  await user.click(within(grid).getByRole("button", { name: "Resume" }));
  await settle();
  expect(take("prepare_resume")).toEqual([{ cmd: "prepare_resume", args: { journalPath: S.resumeJournal } }]);
  expect(S.src?.label).toBe("ZR_CARD_A");
});

test("an unfinished offload can be resumed from its tab", async () => {
  const { S, take, user, panel } = await startApp();
  await user.click(tab("ZR_CARD_A")); // unfinished, and its card is connected
  await settle();
  take();
  await user.click(panel("pSrc").getByRole("button", { name: "Resume" }));
  await settle();
  expect(take("prepare_resume")).toEqual([{ cmd: "prepare_resume", args: { journalPath: S.resumeJournal } }]);
  expect(S.view).toBe("");
  expect(panel("pSrc").getByText(/Picks up where it stopped/)).toBeInTheDocument();
});

test("an offload can be removed from the history", async () => {
  const { take, user } = await startApp();
  const before = tabs().length;
  await user.click(document.querySelector<HTMLElement>('.strip .tab [title="Remove from history"]')!);
  await settle();
  expect(take().map((c) => c.cmd)).toEqual(["forget_history", "history_list"]);
  expect(tabs()).toHaveLength(before - 1);
});

test("re-verify checks a folder, then shows its log and the folder", async () => {
  const { take, user, screen, menu, dialog } = await startApp();
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: /^Re-verify a Folder…/ }));
  await user.click(dialog().getByRole("button", { name: "Choose Folder…" }));
  await settle();
  expect(take().map((c) => c.cmd)).toEqual(["plugin:dialog|open", "reverify_folder"]);
  await runUntil(() => document.getElementById("scrim")?.textContent?.includes("match their recorded checksums") ?? false);
  expect(dialog().getByText(/All 20 files match/)).toBeInTheDocument();

  await user.click(dialog().getByRole("button", { name: /Show in Finder/ }));
  await settle();
  expect(take("reveal")).toEqual([{ cmd: "reveal", args: { path: "/Volumes/SHUTTLE_A/Jobs/2026-Fall" } }]);
  await user.click(dialog().getByRole("button", { name: "Show log" }));
  await settle();
  expect(take("read_text")).toEqual([{ cmd: "read_text", args: { path: "preview.log" } }]);
});
