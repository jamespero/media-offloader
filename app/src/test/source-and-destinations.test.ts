// Choosing what to offload and where to: the source panel and the copy and backup panels.

import { afterEach, expect, test } from "vitest";
import { settle, startApp, stopApp } from "./app";

afterEach(stopApp);

async function withCard() {
  const app = await startApp();
  await app.user.click(app.screen.getByRole("button", { name: /Select a source/ }));
  await app.user.click(app.menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await settle();
  app.take();
  return app;
}

test("the source menu lists connected media and picks one", async () => {
  const { S, take, user, screen, menu } = await startApp();
  await user.click(screen.getByRole("button", { name: /Select a source/ }));
  for (const label of ["ZR_CARD_A", "NINJA2", "F6_SD"]) expect(menu().getByRole("button", { name: new RegExp(label) })).toBeInTheDocument();

  await user.click(menu().getByRole("button", { name: /NINJA2/ }));
  await settle();
  expect(document.querySelector(".menu")).toBeNull();
  expect(take("preview_source")).toEqual([{ cmd: "preview_source", args: { path: "/Volumes/NINJA2", includeSystemFiles: false } }]);
  expect(S.src).toMatchObject({ label: "NINJA2", kind: "drive" });
  expect(S.phase).toBe("ready");
});

test("a folder can be the source, chosen with the system dialog", async () => {
  const { S, take, user, panel, menu } = await withCard();
  await user.click(panel("pSrc").getByRole("button", { name: /ZR_CARD_A/ }));
  await user.click(menu().getByRole("button", { name: "Choose Folder…" }));
  await settle();
  const calls = take();
  expect(calls[0]).toEqual({ cmd: "plugin:dialog|open", args: { options: { directory: true, title: "Choose a folder to offload" } } });
  expect(calls[1]).toEqual({ cmd: "preview_source", args: { path: "/Users/you/Desktop/Day 3 selects", includeSystemFiles: false } });
  expect(S.src).toMatchObject({ kind: "folder", label: "Day 3 selects" });
});

test("Browse… sets the copy destination and works out the folders", async () => {
  // This button silently did nothing when a menu item closed its menu before running.
  const { S, take, user, panel, menu } = await withCard();
  await user.click(panel("pCopy").getByRole("button", { name: "Choose…" }));
  await user.click(menu().getByRole("button", { name: "Browse…" }));
  await settle();
  const calls = take();
  expect(calls[0]).toEqual({ cmd: "plugin:dialog|open", args: { options: { directory: true, title: "Copy media to…" } } });
  expect(calls[1]).toMatchObject({
    cmd: "prepare",
    args: { args: { source: "/Volumes/ZR_CARD_A", copyTo: expect.stringMatching(/^\/Volumes\/SHUTTLE_A\/Jobs\/2026-Fall\/\d\d-\d\d-\d\d\/ZR_CARD_A$/), backupTo: null } },
  });
  expect(S.settings.copyDest).toBe("/Volumes/SHUTTLE_A/Jobs/2026-Fall");
  expect(S.settings.recentCopy).toEqual(["/Volumes/SHUTTLE_A/Jobs/2026-Fall"]);
  expect(panel("pSrc").getByRole("button", { name: "Start" })).toBeEnabled();
});

test("a recent destination can be picked again from the menu", async () => {
  const { S, take, user, panel, menu } = await withCard();
  await user.click(panel("pCopy").getByRole("button", { name: "Choose…" }));
  await user.click(menu().getByRole("button", { name: "Browse…" }));
  await settle();
  S.settings.copyDest = null;
  take();

  await user.click(panel("pCopy").getByRole("button", { name: "Choose…" }));
  await user.click(menu().getByRole("button", { name: "/Volumes/SHUTTLE_A/Jobs/2026-Fall" }));
  await settle();
  expect(S.settings.copyDest).toBe("/Volumes/SHUTTLE_A/Jobs/2026-Fall");
  expect(take("prepare")).toHaveLength(1);
});

test("a backup destination can be added and removed", async () => {
  const { S, take, user, panel, menu } = await withCard();
  await user.click(panel("pBak").getByRole("button", { name: "Choose…" }));
  await user.click(menu().getByRole("button", { name: "Browse…" }));
  await settle();
  expect(take("plugin:dialog|open")[0].args).toEqual({ options: { directory: true, title: "Back up media to…" } });
  expect(S.settings.bakDest).toBe("/Volumes/SHUTTLE_B/Jobs/2026-Fall");

  await user.click(panel("pBak").getByRole("button", { name: "Remove backup destination" }));
  await settle();
  expect(S.settings.bakDest).toBeNull();
  expect(panel("pBak").getByText("Select a location to save a second copy of all media files.")).toBeInTheDocument();
});

test("Start needs a copy destination first", async () => {
  const { panel } = await withCard();
  const start = panel("pSrc").getByRole("button", { name: "Start" });
  expect(start).toBeDisabled();
  expect(start).toHaveAttribute("title", "Choose where to copy first.");
});
