// Changing settings from the Settings sheet.

import { selectSource } from "./source";
import { S, running, saveSettings, type Settings } from "./state.svelte.ts";

export type BoolSetting = { [K in keyof Settings]: Settings[K] extends boolean ? K : never }[keyof Settings];

/** Flips an on/off setting. Copying hidden files changes what's on the source, so it's read again. */
export function toggleSetting(k: BoolSetting) {
  S.settings[k] = !S.settings[k];
  saveSettings();
  if (k === "includeSystemFiles" && S.src && !running()) selectSource(S.src.path);
}

/** Checksum: xxHash64 alone, or with MD5 as well. */
export function setMd5(on: boolean) {
  S.settings.md5 = on;
  saveSettings();
}

/** Copy mode: sequential, or fan-out (read the card once, write every drive). */
export function setFanOut(on: boolean) {
  S.settings.fanOut = on;
  saveSettings();
}
