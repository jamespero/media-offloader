// Keyboard shortcuts for the whole window. Escape closes whatever is open.

import { closeMenus } from "./components/Menu/menuState.svelte.ts";
import { closeOverlay } from "./components/Overlay/overlayState.svelte.ts";
import { settings } from "./components/Settings/settingsState.svelte.ts";
import { openRunLog, reverifyIntro } from "./dialogs";
import { newOffload } from "./offload";
import { S } from "./state.svelte.ts";

export function onShortcut(e: KeyboardEvent) {
  if (e.key === "Escape") {
    closeMenus();
    closeOverlay();
    return;
  }
  const cmd = e.ctrlKey || e.metaKey;
  if (!cmd || (e.target as Element).closest("input")) return;
  if (e.key === "n") {
    e.preventDefault();
    newOffload();
  } else if (e.key === "l" && S.report?.logPath) {
    e.preventDefault();
    openRunLog();
  } else if (e.key === "r") {
    e.preventDefault();
    reverifyIntro();
  } else if (e.key === ",") {
    e.preventDefault();
    settings();
  }
}
