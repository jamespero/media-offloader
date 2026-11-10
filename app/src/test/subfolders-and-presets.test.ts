// The subfolder chips under the copy destination, and presets that save them.

import { afterEach, expect, test } from "vitest";
import { settle, startApp, stopApp } from "./app";

afterEach(stopApp);

async function ready() {
  const app = await startApp();
  await app.user.click(app.screen.getByRole("button", { name: /Select a source/ }));
  await app.user.click(app.menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await app.user.click(app.panel("pCopy").getByRole("button", { name: "Choose…" }));
  await app.user.click(app.menu().getByRole("button", { name: "Browse…" }));
  await settle();
  app.take();
  return app;
}

const chips = () => [...document.querySelectorAll("#pCopy .chip")].map((c) => c.textContent!.replace(/\s+/g, " ").trim());
const lastCopyTo = (take: (cmd: string) => { args: Record<string, unknown> }[]) =>
  (take("prepare").at(-1)!.args.args as { copyTo: string }).copyTo;

test("subfolders can be added, edited, moved and removed", async () => {
  const { S, take, user, panel, menu } = await ready();
  expect(chips()).toEqual(["Now YY-MM-DD", "Source Name"]);

  await user.click(panel("pCopy").getByRole("button", { name: "Add subfolder" }));
  await user.click(menu().getByRole("button", { name: /Reel \/ Roll #/ }));
  await settle();
  expect(chips()).toEqual(["Now YY-MM-DD", "Source Name", "Reel A###"]);
  expect(lastCopyTo(take)).toMatch(/\/ZR_CARD_A\/A001$/);

  await user.click(panel("pCopy").getByRole("button", { name: /Reel A###/ }));
  await user.click(menu().getByRole("button", { name: /^B001/ }));
  await settle();
  expect(S.settings.tokens[2]).toEqual({ t: "reel", v: "B" });
  expect(lastCopyTo(take)).toMatch(/\/B001$/);

  await user.click(panel("pCopy").getByRole("button", { name: /Reel B###/ }));
  await user.click(menu().getByRole("button", { name: "Move Left" }));
  await settle();
  expect(chips()).toEqual(["Now YY-MM-DD", "Reel B###", "Source Name"]);

  await user.click(panel("pCopy").getByRole("button", { name: /Now YY-MM-DD/ }));
  await user.click(menu().getByRole("button", { name: /^YYYY-MM-DD/ }));
  await settle();
  expect(S.settings.tokens[0]).toEqual({ t: "now", fmt: "YYYY-MM-DD" });

  await user.click(panel("pCopy").getByRole("button", { name: /Source Name/ }));
  await user.click(menu().getByRole("button", { name: "Remove" }));
  await settle();
  expect(chips()).toEqual(["Now YYYY-MM-DD", "Reel B###"]);
  expect(lastCopyTo(take)).toMatch(/\/2026-Fall\/\d{4}-\d\d-\d\d\/B001$/);
});

test("custom text and the next reel number are typed into the chip's menu", async () => {
  const { S, user, panel, menu } = await ready();
  await user.click(panel("pCopy").getByRole("button", { name: "Add subfolder" }));
  await user.click(menu().getByRole("button", { name: /Custom Text/ }));
  await user.click(panel("pCopy").getByRole("button", { name: /^text/ }));
  const text = menu().getByRole("textbox");
  await user.clear(text);
  await user.type(text, "B-cam{Enter}");
  expect(S.settings.tokens.at(-1)).toEqual({ t: "text", v: "B-cam" });
  expect(document.querySelector(".menu")).toBeNull();

  await user.click(panel("pCopy").getByRole("button", { name: "Add subfolder" }));
  await user.click(menu().getByRole("button", { name: /Reel \/ Roll #/ }));
  await user.click(panel("pCopy").getByRole("button", { name: /Reel A###/ }));
  const next = menu().getByRole("spinbutton");
  await user.clear(next);
  await user.type(next, "7");
  expect(S.settings.reels).toEqual({ A: 7 });
});

test("a preset saves the subfolders, and choosing it brings them back", async () => {
  const { S, user, screen, panel, menu, dialog } = await ready();
  await user.click(screen.getByRole("button", { name: /^Preset/ }));
  await user.click(menu().getByRole("button", { name: "Save Current as Preset…" }));
  const name = dialog().getByRole("textbox");
  expect(name).toHaveValue("ZR_CARD_A");
  await user.clear(name);
  await user.type(name, "  Nikon ZR {Enter}");
  expect(S.settings.presets).toEqual([{ id: expect.any(String), name: "Nikon ZR", tokens: [{ t: "now", fmt: "YY-MM-DD" }, { t: "source" }] }]);
  expect(screen.getByText("Preset saved")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /^Preset/ })).toHaveTextContent("Nikon ZR");

  await user.click(panel("pCopy").getByRole("button", { name: /Source Name/ }));
  await user.click(menu().getByRole("button", { name: "Remove" }));
  expect(screen.getByRole("button", { name: /^Preset/ })).toHaveTextContent("Custom");

  await user.click(screen.getByRole("button", { name: /^Preset/ }));
  await user.click(menu().getByRole("button", { name: /Nikon ZR/ }));
  await settle();
  expect(chips()).toEqual(["Now YY-MM-DD", "Source Name"]);
});

test("presets are managed in Settings, with the Import # counter", async () => {
  const { S, user, screen, menu, dialog } = await ready();
  await user.click(screen.getByRole("button", { name: /^Preset/ }));
  await user.click(menu().getByRole("button", { name: "Save Current as Preset…" }));
  await user.click(dialog().getByRole("button", { name: "Save" }));
  await user.click(screen.getByRole("button", { name: /^Preset/ }));
  await user.click(menu().getByRole("button", { name: "Manage Presets…" }));
  expect(dialog().getByText("ZR_CARD_A")).toBeInTheDocument();

  await user.click(dialog().getByRole("button", { name: "Raise" }));
  expect(S.settings.importNo).toBe(2);
  await user.click(dialog().getByRole("button", { name: "Lower" }));
  await user.click(dialog().getByRole("button", { name: "Lower" }));
  expect(S.settings.importNo).toBe(1);

  await user.click(dialog().getByRole("button", { name: "Delete preset" }));
  expect(S.settings.presets).toEqual([]);
});
