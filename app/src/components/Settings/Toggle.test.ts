import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test } from "vitest";
import { S } from "../../state.svelte.ts";
import Toggle from "./Toggle.svelte";

beforeEach(() => {
  S.settings.sound = true;
  S.settings.md5List = false;
});

test("flips its setting and saves it", async () => {
  render(Toggle, { on: S.settings.sound, setting: "sound", label: "Sound" });
  await userEvent.click(screen.getByRole("button", { name: "Sound" }));
  expect(S.settings.sound).toBe(false);
  expect(JSON.parse(localStorage.getItem("media-offloader.settings.v1")!).sound).toBe(false);
});

test("a locked toggle doesn't change", async () => {
  render(Toggle, { on: false, setting: "md5List", label: "List", lock: true });
  await userEvent.click(screen.getByRole("button", { name: "List" }));
  expect(S.settings.md5List).toBe(false);
});

test("shows on or off to assistive tech", () => {
  render(Toggle, { on: true, label: "On" });
  expect(screen.getByRole("button", { name: "On" })).toHaveAttribute("aria-pressed", "true");
});
