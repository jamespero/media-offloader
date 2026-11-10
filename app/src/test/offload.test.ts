// Running an offload against the simulated engine: starting, stopping, and what's offered after.

import { afterEach, expect, test } from "vitest";
import { runUntil, settle, startApp, stopApp } from "./app";

afterEach(stopApp);

async function readyToStart(search = "") {
  const app = await startApp(search);
  await app.user.click(app.screen.getByRole("button", { name: /Select a source/ }));
  await app.user.click(app.menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await app.user.click(app.panel("pCopy").getByRole("button", { name: "Choose…" }));
  await app.user.click(app.menu().getByRole("button", { name: "Browse…" }));
  await app.user.click(app.panel("pBak").getByRole("button", { name: "Choose…" }));
  await app.user.click(app.menu().getByRole("button", { name: "Browse…" }));
  await settle();
  app.take();
  return app;
}

test("an offload copies, backs up and verifies, then offers its log and copy", async () => {
  const { S, take, user, screen, panel, menu, dialog } = await readyToStart();
  await user.click(panel("pSrc").getByRole("button", { name: "Start" }));
  await settle(100);
  expect(take().map((c) => c.cmd)).toEqual(["prepare", "thumbs_request", "start"]);
  expect(S.phase).toBe("copying");
  expect(panel("pSrc").getByText("Copying from this source")).toBeInTheDocument();

  // Stop asks first; Keep going carries on.
  await user.click(panel("pCopy").getByRole("button", { name: "Stop" }));
  expect(dialog().getByText("Stop this offload?")).toBeInTheDocument();
  await user.click(dialog().getByRole("button", { name: "Keep going" }));
  expect(take("cancel")).toEqual([]);

  await runUntil(() => S.phase === "done");
  expect(panel("pSrc").getByText("Copied & verified")).toBeInTheDocument();
  expect(panel("pCopy").getByText("100%")).toBeInTheDocument();
  expect(panel("pBak").getByText("100%")).toBeInTheDocument();
  take();

  expect(panel("pCopy").getByRole("button", { name: /in 55 files · copied & checksum verified/ })).toBeInTheDocument();
  await user.click(panel("pCopy").getByTitle("Open the log for this offload"));
  await settle();
  expect(take("read_text")).toHaveLength(1);
  expect(dialog().getByText("Transfer log")).toBeInTheDocument();
  await user.click(dialog().getByRole("button", { name: /Show in Finder/ }));
  await settle();
  expect(take("reveal")).toHaveLength(1);
  await user.click(dialog().getByRole("button", { name: "Close" }));

  await user.keyboard("{Control>}l{/Control}");
  await settle();
  expect(take("read_text")).toHaveLength(1);
  await user.keyboard("{Escape}");

  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: /Copy in Finder/ }));
  await settle();
  expect(take("reveal")).toEqual([{ cmd: "reveal", args: { path: S.dests![0].root } }]);
});

test("double-clicking a thumbnail shows the original, or the copy once it's verified", async () => {
  const { S, take, user, panel } = await readyToStart();
  const first = () => document.querySelector<HTMLElement>(".thumb")!;
  await user.dblClick(first());
  await settle();
  expect(take("reveal")).toEqual([{ cmd: "reveal", args: { path: `/Volumes/ZR_CARD_A/${S.files[0].srcRel}` } }]);

  await user.click(panel("pSrc").getByRole("button", { name: "Start" }));
  await runUntil(() => S.phase === "done");
  take();
  await user.dblClick(first());
  await settle();
  expect(take("reveal")).toEqual([{ cmd: "reveal", args: { path: `${S.dests![0].root}/${S.files[0].destRel}` } }]);
});

test("Stop cancels the offload and keeps what's verified", async () => {
  const { S, take, user, panel, dialog } = await readyToStart();
  await user.click(panel("pSrc").getByRole("button", { name: "Start" }));
  await runUntil(() => S.files.some((f) => f.state === "copied"));
  await user.click(panel("pCopy").getByRole("button", { name: "Stop" }));
  await user.click(dialog().getByRole("button", { name: "Stop" }));
  expect(take("cancel")).toHaveLength(1);
  await runUntil(() => S.phase !== "copying");
  expect(S.phase).toBe("ready");
  expect(S.stoppedKept).toBeGreaterThan(0);
});

test("New Offload clears the source, from the menu or Ctrl+N", async () => {
  const { S, user, screen, menu } = await readyToStart();
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: /^New Offload/ }));
  expect(S.src).toBeNull();

  await user.click(screen.getByRole("button", { name: /Select a source/ }));
  await user.click(menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await settle();
  await user.keyboard("{Control>}n{/Control}");
  expect(S.src).toBeNull();
});

test("failed files are shown with a problem panel and its log", async () => {
  const { S, take, user, panel, dialog } = await readyToStart("?fail");
  await user.click(panel("pSrc").getByRole("button", { name: "Start" }));
  await runUntil(() => S.phase === "error");
  expect(panel("pCopy").getByText("copy incomplete")).toBeInTheDocument();
  take();
  await user.click(panel("pCopy").getByRole("button", { name: "Show log" }));
  await settle();
  expect(take("read_text")).toHaveLength(1);
  expect(dialog().getByText("Transfer log")).toBeInTheDocument();
});
