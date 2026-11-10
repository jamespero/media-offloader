// Presets: a saved subfolder pattern, one per camera or recorder. Destinations
// aren't part of a preset: they're the project's
// folders, chosen once per project, and a preset's pattern goes under them. The active
// preset is whichever one matches the current pattern; any change makes it "Custom" until
// saved again.

import { closeOverlay } from "./components/Overlay/overlayState.svelte.ts";
import { schedulePrepare } from "./destinations";
import { S, plain, running } from "./state.svelte.ts";
import type { Token } from "./tokens";

export interface Preset {
  id: string;
  name: string;
  tokens: Token[];
}

const sameTokens = (a: Token[], b: Token[]) => JSON.stringify(a) === JSON.stringify(b);

export function activePreset(): Preset | undefined {
  const s = S.settings;
  return s.presets.find((p) => sameTokens(p.tokens, s.tokens));
}

export function applyPreset(id: string) {
  const p = S.settings.presets.find((x) => x.id === id);
  if (!p || running()) return;
  S.settings.tokens = plain(p.tokens);
}

/** Switches to a preset's subfolders and works out the destinations again. */
export function usePreset(id: string) {
  applyPreset(id);
  S.dests = null;
  schedulePrepare();
}

/** Saves the current setup under `name`; an existing preset with that name is updated. */
export function saveCurrentAs(name: string) {
  name = name.trim();
  if (!name) return false;
  const s = S.settings;
  const preset: Preset = { id: crypto.randomUUID(), name, tokens: plain(s.tokens) };
  const i = s.presets.findIndex((p) => p.name.toLowerCase() === name.toLowerCase());
  if (i >= 0) s.presets[i] = { ...preset, id: s.presets[i].id };
  else s.presets.push(preset);
  closeOverlay();
  return true;
}

export function deletePreset(id: string) {
  S.settings.presets = S.settings.presets.filter((p) => p.id !== id);
}
