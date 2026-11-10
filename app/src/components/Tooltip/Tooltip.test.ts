import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { flushSync } from "svelte";
import { expect, test } from "vitest";
import Thumb from "../Grid/Thumb.svelte";
import { S, fileView } from "../../state.svelte.ts";
import Tooltip from "./Tooltip.svelte";

const file = () => fileView("DCIM/100/DSC_0001.MOV", "DCIM/100/DSC_0001.MOV", 2_000_000_000, Date.parse("2026-10-05T08:14:00"));
const tip = () => document.getElementById("tip")!;

test("hovering a thumbnail shows its file; leaving hides it", async () => {
  render(Tooltip);
  render(Thumb, { file: file() });
  const thumb = document.querySelector<HTMLElement>(".thumb")!;

  await userEvent.hover(thumb);
  expect(tip()).toHaveClass("on");
  expect(tip()).toHaveTextContent("DSC_0001.MOV");
  expect(tip()).toHaveTextContent("2.0 GB");
  expect(tip()).toHaveTextContent("DCIM/100/DSC_0001.MOV");
  expect(tip()).toHaveTextContent("Not copied yet");

  await userEvent.unhover(thumb);
  expect(tip()).not.toHaveClass("on");
});

test("follows the file's state while hovered", async () => {
  render(Tooltip);
  // As in the app: the file lives in the reactive state, and its tile is given that.
  S.files = [file()];
  const f = S.files[0];
  render(Thumb, { file: f });
  await userEvent.hover(document.querySelector<HTMLElement>(".thumb")!);
  f.state = "copied";
  flushSync();
  expect(tip()).toHaveTextContent("Copied & verified");
  f.state = "failed";
  f.failure = "checksum mismatch";
  flushSync();
  expect(tip()).toHaveTextContent("checksum mismatch");
});

test("a thumbnail removed while hovered takes the tooltip with it", async () => {
  render(Tooltip);
  const { unmount } = render(Thumb, { file: file() });
  await userEvent.hover(document.querySelector<HTMLElement>(".thumb")!);
  expect(tip()).toHaveClass("on");
  unmount();
  flushSync();
  expect(tip()).not.toHaveClass("on");
  expect(screen.queryByText("DSC_0001.MOV")).toBeNull();
});
