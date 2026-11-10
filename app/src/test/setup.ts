// Runs before every test file.

import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach } from "vitest";

// jsdom does no layout, so there's nothing to observe; Svelte's size bindings need the API.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};

afterEach(() => {
  clearMocks();
  localStorage.clear();
});
