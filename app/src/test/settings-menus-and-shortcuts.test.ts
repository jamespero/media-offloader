// The Settings sheet, the main menu, keyboard shortcuts, and how menus and dialogs close.

import { afterEach, expect, test, vi } from "vitest";
import { settle, startApp, stopApp } from "./app";

afterEach(stopApp);

const saved = () => JSON.parse(localStorage.getItem("media-offloader.settings.v1") ?? "{}");

test("settings toggles change the setting and are saved", async () => {
  const { S, user, screen, menu, dialog } = await startApp();
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: /^Settings…/ }));

  for (const [label, key] of [
    ["Auto-select inserted media", "autoSelect"],
    ["Keep the computer awake", "keepAwake"],
    ["Install updates automatically", "autoUpdate"],
  ] as const) {
    const before = S.settings[key];
    await user.click(dialog().getByRole("button", { name: label }));
    expect(S.settings[key]).toBe(!before);
    expect(saved()[key]).toBe(!before);
  }
  // Eject is hidden for now.
  expect(dialog().queryByRole("button", { name: "Eject source when verified" })).toBeNull();
});

test("copying hidden files reads the source again", async () => {
  const { take, user, screen, menu, dialog } = await startApp();
  await user.click(screen.getByRole("button", { name: /Select a source/ }));
  await user.click(menu().getByRole("button", { name: /ZR_CARD_A/ }));
  await settle();
  take();
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: /^Settings…/ }));
  await user.click(dialog().getByRole("button", { name: "Copy hidden system files" }));
  await settle();
  expect(take("preview_source")).toEqual([{ cmd: "preview_source", args: { path: "/Volumes/ZR_CARD_A", includeSystemFiles: true } }]);
});

test("checksum and copy mode are two-way choices; locked toggles don't change", async () => {
  const { S, user, dialog } = await startApp();
  await user.keyboard("{Control>},{/Control}");
  await user.click(dialog().getByRole("button", { name: "Verification" }));
  await user.click(dialog().getByRole("button", { name: "xxHash64 + MD5" }));
  expect(S.settings.md5).toBe(true);
  await user.click(dialog().getByRole("button", { name: "Write .md5 checksum list" }));
  expect(S.settings.md5List).toBe(true);
  await user.click(dialog().getByRole("button", { name: "Verify from disk, not cache" }));
  await user.click(dialog().getByRole("button", { name: "xxHash64" }));
  expect(S.settings.md5).toBe(false);
  // Without MD5 the .md5 list is locked off.
  const md5List = dialog().getByRole("button", { name: "Write .md5 checksum list" });
  expect(md5List).toHaveAttribute("aria-pressed", "false");
  await user.click(md5List);
  expect(S.settings.md5List).toBe(true);

  await user.click(dialog().getByRole("button", { name: "Transfers" }));
  await user.click(dialog().getByRole("button", { name: "Fan-out" }));
  expect(S.settings.fanOut).toBe(true);
  await user.click(dialog().getByRole("button", { name: "Sequential" }));
  expect(S.settings.fanOut).toBe(false);
});

test("Show logs folder opens it", async () => {
  const { take, user, screen, menu, dialog } = await startApp();
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: "Show Logs Folder" }));
  await settle();
  expect(take().map((c) => c.cmd)).toEqual(["log_dir", "open_folder"]);

  await user.keyboard("{Control>},{/Control}");
  await user.click(dialog().getByRole("button", { name: "Logs" }));
  await settle();
  take();
  await user.click(dialog().getByRole("button", { name: "Show" }));
  await settle();
  expect(take("open_folder")).toEqual([{ cmd: "open_folder", args: { path: "~/Library/Logs/Media Offloader" } }]);
});

test("About links to the repository", async () => {
  const { take, user, screen, menu, dialog } = await startApp();
  const open = vi.spyOn(window, "open").mockReturnValue(null);
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(menu().getByRole("button", { name: "About Media Offloader" }));
  await settle();
  await user.click(dialog().getByRole("button", { name: "View on GitHub" }));
  await settle();
  expect(take("open_repo")).toHaveLength(1);
  expect(open).toHaveBeenCalledWith("https://github.com/jamespero/media-offloader", "_blank");
  await user.click(dialog().getByRole("button", { name: "Close" }));
  expect(document.getElementById("scrim")).toBeNull();
});

test("a menu closes on Escape, a click outside it, or a resize", async () => {
  const { user, screen } = await startApp();
  const menuOpen = () => document.querySelector(".menu") !== null;

  await user.click(screen.getByRole("button", { name: "Menu" }));
  expect(menuOpen()).toBe(true);
  await user.keyboard("{Escape}");
  expect(menuOpen()).toBe(false);

  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(document.getElementById("gridwrap")!);
  expect(menuOpen()).toBe(false);

  await user.click(screen.getByRole("button", { name: "Menu" }));
  window.dispatchEvent(new Event("resize"));
  await settle(10);
  expect(menuOpen()).toBe(false);

  // Clicking another menu's button switches to that menu.
  await user.click(screen.getByRole("button", { name: "Menu" }));
  await user.click(screen.getByRole("button", { name: /Select a source/ }));
  expect(document.querySelector(".menu")).toHaveTextContent("Connected media");
});

test("keyboard shortcuts open Settings and Re-verify, and Escape closes them", async () => {
  const { user, dialog } = await startApp();
  await user.keyboard("{Control>},{/Control}");
  expect(dialog().getByText("settings")).toBeInTheDocument();
  await user.keyboard("{Escape}");
  expect(document.getElementById("scrim")).toBeNull();

  await user.keyboard("{Control>}r{/Control}");
  expect(dialog().getByText("Re-verify a folder")).toBeInTheDocument();
  await user.click(dialog().getByRole("button", { name: "Cancel" }));
  expect(document.getElementById("scrim")).toBeNull();

  // No log yet, so Ctrl+L does nothing.
  await user.keyboard("{Control>}l{/Control}");
  expect(document.getElementById("scrim")).toBeNull();
});
