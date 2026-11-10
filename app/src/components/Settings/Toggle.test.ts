import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { expect, test, vi } from "vitest";
import Toggle from "./Toggle.svelte";

test("flips on a click and reports the new value", async () => {
  const onchange = vi.fn();
  render(Toggle, { on: false, label: "Sound", onchange });
  const toggle = screen.getByRole("button", { name: "Sound" });
  await userEvent.click(toggle);
  expect(toggle).toHaveAttribute("aria-pressed", "true");
  expect(onchange).toHaveBeenLastCalledWith(true);
  await userEvent.click(toggle);
  expect(toggle).toHaveAttribute("aria-pressed", "false");
  expect(onchange).toHaveBeenLastCalledWith(false);
});

test("a locked or disabled toggle doesn't change", async () => {
  const onchange = vi.fn();
  render(Toggle, { on: true, label: "Always", lock: true, onchange });
  render(Toggle, { on: false, label: "Busy", disabled: true, onchange });
  await userEvent.click(screen.getByRole("button", { name: "Always" }));
  await userEvent.click(screen.getByRole("button", { name: "Busy" }));
  expect(screen.getByRole("button", { name: "Always" })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByRole("button", { name: "Busy" })).toHaveAttribute("aria-pressed", "false");
  expect(onchange).not.toHaveBeenCalled();
});
