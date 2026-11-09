// Checking for updates: shortly after launch and every few hours, against the latest
// GitHub release. A new version downloads quietly in the background (a few MB, network
// only, so it's fine during an offload). Only once it's downloaded, and nothing is running
// and no other dialog is open, does the app ask to install it. Installing restarts the
// app, which is why the question waits. Updates are signed; the updater rejects anything
// that isn't.
//
// With "Install updates automatically" on, there's no question: an update found at launch
// installs right away (the app restarts before any offload starts), and one found later
// waits for the next launch.

import { check, type Update } from "@tauri-apps/plugin-updater";
import { openedOverlay, overlay, whenOverlayCloses } from "./components/Overlay/overlayState.svelte.ts";
import UpdateDialog from "./components/UpdateDialog/UpdateDialog.svelte";
import { reverifyRunning } from "./reverify.svelte.ts";
import { S, running } from "./state.svelte.ts";

const FIRST_CHECK_MS = 5_000;
const RECHECK_MS = 6 * 60 * 60 * 1000;
const IDLE_POLL_MS = 15_000;
/** How long after launch an automatic update may still install; later ones wait for the next launch. */
const LAUNCH_WINDOW_MS = 2 * 60 * 1000;
const launchedAt = Date.now();

/** Downloaded and ready to install. */
let ready: Update | null = null;
/** Checking or downloading right now. */
let busy = false;
/** The version the user said "Not now" to; asked again at the next launch. */
let declined: string | null = null;

const idle = () => !running() && !reverifyRunning() && !openedOverlay();

async function checkAndDownload() {
  if (busy || ready) return;
  busy = true;
  try {
    const u = await check();
    if (u && u.version !== declined) {
      await u.download();
      ready = u;
    }
  } catch {
    // Offline (common on set), no release reachable, or the download broke off:
    // try again at the next check.
  } finally {
    busy = false;
  }
}

function maybePrompt() {
  if (!ready || ready.version === declined || !idle()) return;
  const auto = S.settings.autoUpdate;
  if (auto && Date.now() - launchedAt >= LAUNCH_WINDOW_MS) return;
  const update = ready;
  overlay(UpdateDialog, { update, auto });
  // "Not now", Escape and clicking outside all close the dialog; the update stays
  // downloaded, but isn't offered again until the next launch.
  whenOverlayCloses(() => {
    declined = update.version;
  });
}

export function initUpdates() {
  setTimeout(() => void checkAndDownload().then(maybePrompt), FIRST_CHECK_MS);
  setInterval(() => void checkAndDownload(), RECHECK_MS);
  setInterval(maybePrompt, IDLE_POLL_MS);
}
