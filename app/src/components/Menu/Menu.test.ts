import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test, vi } from "vitest";
import Menu from "./Menu.svelte";
import MenuItem from "./MenuItem.svelte";
import { closeMenus, openMenu, openedMenu } from "./menuState.svelte.ts";

beforeEach(() => {
  closeMenus();
  document.body.replaceChildren();
});

function openWith(onclick = () => {}) {
  render(Menu);
  const anchor = document.createElement("button");
  anchor.textContent = "outside";
  document.body.append(anchor);
  openMenu(MenuItem, { label: "Do it", onclick }, anchor);
  return anchor;
}

test("a menu item runs its action while the menu is still open, then closes it", async () => {
  // Closing first takes away the menu's props, which an action like DestMenu's still reads.
  let openDuring: unknown = "not run";
  openWith(() => (openDuring = openedMenu()));
  await userEvent.click(await screen.findByRole("button", { name: "Do it" }));
  expect(openDuring).not.toBeNull();
  expect(openedMenu()).toBeNull();
  expect(document.querySelector(".menu")).toBeNull();
});

test("a click outside the menu closes it; a click inside doesn't", async () => {
  const anchor = openWith();
  const menu = await vi.waitFor(() => {
    const m = document.querySelector<HTMLElement>(".menu");
    if (!m) throw new Error("menu not open yet");
    return m;
  });
  await fireEvent.click(menu);
  expect(openedMenu()).not.toBeNull();
  await fireEvent.click(anchor);
  expect(openedMenu()).toBeNull();
});

test("resizing the window closes the menu", async () => {
  openWith();
  await vi.waitFor(() => expect(document.querySelector(".menu")).not.toBeNull());
  await fireEvent(window, new Event("resize"));
  expect(openedMenu()).toBeNull();
});

test("a disabled item does nothing", async () => {
  const onclick = vi.fn();
  render(MenuItem, { label: "Off", onclick, disabled: true });
  await userEvent.click(screen.getByRole("button", { name: "Off" }));
  expect(onclick).not.toHaveBeenCalled();
});
