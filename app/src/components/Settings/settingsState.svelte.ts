// Opens the settings sheet, or switches tab if it's already open.

import { openedOverlay, overlay } from "../Overlay/overlayState.svelte.ts";
import Settings from "./Settings.svelte";

let tab = $state("general");

export const settingsTab = () => tab;

export function settings(next = "general") {
  tab = next;
  if (openedOverlay()?.component !== Settings) overlay(Settings);
}
