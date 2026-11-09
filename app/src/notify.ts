// Telling you it's done without watching the screen: a short chime and a system
// notification. The chime is synthesized, so there's no audio file to bundle.

import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { S } from "./state.svelte.ts";

/** Two rising notes when everything verified; two falling, lower notes for a problem. */
export function chime(ok: boolean) {
  if (!S.settings.sound) return;
  try {
    const ctx = new AudioContext();
    const notes: [number, number][] = ok ? [[880, 0], [1318.5, 0.14]] : [[392, 0], [261.6, 0.24]];
    for (const [freq, at] of notes) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      const t = ctx.currentTime + at;
      osc.type = "sine";
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0.0001, t);
      gain.gain.exponentialRampToValueAtTime(0.22, t + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.7);
      osc.connect(gain).connect(ctx.destination);
      osc.start(t);
      osc.stop(t + 0.75);
    }
    setTimeout(() => ctx.close(), 1500);
  } catch {
    /* no audio device */
  }
}

/** A system notification, only when the window isn't in front (the in-app toast covers that). */
export async function notify(title: string, body: string) {
  if (!S.settings.notify || document.hasFocus()) return;
  try {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    if (granted) sendNotification({ title, body });
  } catch {
    /* notifications unavailable */
  }
}
