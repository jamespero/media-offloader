import { mockIPC } from "@tauri-apps/api/mocks";
import { beforeEach, expect, test } from "vitest";
import { revealFile } from "./places";
import { S, fileView } from "./state.svelte.ts";

let revealed: string[] = [];

beforeEach(() => {
  revealed = [];
  mockIPC((cmd, args) => {
    if (cmd === "reveal") revealed.push((args as { path: string }).path);
  });
  S.view = "";
  S.detail = null;
  S.src = { path: "/Volumes/CARD/", label: "CARD", kind: "card", media: "" };
  S.dests = [{ role: "copy", requested: "/Copy/CARD", root: "/Copy/CARD", conflict: null } as never];
});

const f = (state: (typeof S.files)[number]["state"]) => ({ ...fileView("Renamed/A.MOV", "DCIM/A.MOV", 1, null), state });
const flush = () => new Promise((r) => setTimeout(r));

test("a file not copied yet is shown on the source, without doubling the root's separator", async () => {
  revealFile(f("pending"));
  await flush();
  expect(revealed).toEqual(["/Volumes/CARD/DCIM/A.MOV"]);
});

test("a copied file is shown in the copy, under its destination name", async () => {
  for (const state of ["copied", "backing", "backed"] as const) revealFile(f(state));
  await flush();
  expect(revealed).toEqual(["/Copy/CARD/Renamed/A.MOV", "/Copy/CARD/Renamed/A.MOV", "/Copy/CARD/Renamed/A.MOV"]);
});

test("in a past offload, the copy and source come from its journal", async () => {
  S.view = "job.jsonl";
  S.detail = { item: { roots: ["/Old/Copy"], source: "/Volumes/OLD" } } as never;
  revealFile(f("copied"));
  revealFile(f("failed"));
  await flush();
  expect(revealed).toEqual(["/Old/Copy/Renamed/A.MOV", "/Volumes/OLD/DCIM/A.MOV"]);
});
