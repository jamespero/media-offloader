// The update dialog, offered when a new version has downloaded (the preview's `?update`).

import { afterEach, expect, test } from "vitest";
import { runUntil, startApp, stopApp } from "./app";

afterEach(stopApp);

test("the update dialog can turn on automatic updates", async () => {
  const { S, user, dialog } = await startApp("?update");
  await runUntil(() => document.getElementById("scrim")?.textContent?.includes("Update ready") ?? false, 30_000);
  expect(dialog().getByText(/9\.9\.9 has been downloaded/)).toBeInTheDocument();

  const auto = dialog().getByRole("button", { name: "Install updates automatically from now on" });
  expect(auto).toHaveAttribute("aria-pressed", "false");
  await user.click(auto);
  expect(S.settings.autoUpdate).toBe(true);
  expect(JSON.parse(localStorage.getItem("media-offloader.settings.v1")!).autoUpdate).toBe(true);
  expect(auto).toHaveAttribute("aria-pressed", "true");
});
