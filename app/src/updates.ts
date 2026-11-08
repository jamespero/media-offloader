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
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "./api";
import { closeOverlay, overlay, whenOverlayCloses } from "./menus";
import { esc } from "./render";
import { reverifyRunning } from "./reverify";
import { S, running, saveSettings } from "./state";

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

const idle = () => !running() && !reverifyRunning() && !document.getElementById("scrim");

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
  if (!S.settings.autoUpdate) prompt(ready);
  else if (Date.now() - launchedAt < LAUNCH_WINDOW_MS) autoInstall(ready);
}

/** Automatic updates: say what's happening for a moment, then install and restart. */
function autoInstall(u: Update) {
  const o = overlay(`<div class="dialog"><h3>Updating</h3>
    <p>Installing Media Offloader ${esc(u.version)}. The app restarts in a moment.</p>
    <p id="updMsg" style="color:var(--color-text-tertiary)">Automatic updates can be turned off in Settings → General.</p>
    <div class="acts"><button class="btn" id="updLater">Not now</button><button class="btn primary" id="updGo" disabled>Installing…</button></div></div>`);
  whenOverlayCloses(() => {
    declined = u.version;
  });
  o.querySelector<HTMLButtonElement>("#updLater")!.onclick = closeOverlay;
  setTimeout(() => {
    if (declined !== u.version && o.isConnected) void install(u, o);
  }, 2500);
}

function prompt(u: Update) {
  const o = overlay(`<div class="dialog"><h3>Update ready</h3>
    <p>Media Offloader ${esc(u.version)} has been downloaded. You have ${esc(u.currentVersion)}.</p>
    <p id="updMsg">Installing takes a few seconds and restarts the app. Your settings and presets are kept.</p>
    <div class="set updauto"><div class="l"><b>Install updates automatically from now on</b><span>As the app starts, before you begin. Never during an offload.</span></div><button class="tog" id="updAuto" aria-pressed="false"></button></div>
    <div class="acts"><button class="btn" id="updNotes">What's new</button><button class="btn" id="updLater">Not now</button><button class="btn primary" id="updGo">Update and relaunch</button></div></div>`);
  // "Not now", Escape and clicking outside all close the dialog; the update stays
  // downloaded, but isn't offered again until the next launch.
  whenOverlayCloses(() => {
    declined = u.version;
  });
  const auto = o.querySelector<HTMLButtonElement>("#updAuto")!;
  auto.onclick = () => {
    S.settings.autoUpdate = !S.settings.autoUpdate;
    saveSettings();
    auto.classList.toggle("on", S.settings.autoUpdate);
    auto.setAttribute("aria-pressed", String(S.settings.autoUpdate));
  };
  o.querySelector<HTMLButtonElement>("#updNotes")!.onclick = () => api.openRelease(u.version).catch(() => {});
  o.querySelector<HTMLButtonElement>("#updLater")!.onclick = closeOverlay;
  o.querySelector<HTMLButtonElement>("#updGo")!.onclick = () => install(u, o);
}

async function install(u: Update, o: HTMLElement) {
  // The dialog only shows when idle and covers the window, but check once more:
  // restarting mid-offload is the one thing this must never do.
  if (running() || reverifyRunning()) return;
  const msg = o.querySelector<HTMLElement>("#updMsg")!;
  const go = o.querySelector<HTMLButtonElement>("#updGo")!;
  const buttons = o.querySelectorAll<HTMLButtonElement>("button");
  buttons.forEach((b) => (b.disabled = true));
  go.textContent = "Installing…";
  try {
    await u.install(); // on Windows the installer closes the app and reopens it itself
    await relaunch();
  } catch (err) {
    msg.textContent = `The update didn't install (${err}). Try again, or download it from the releases page.`;
    msg.style.color = "var(--color-status-problem)";
    buttons.forEach((b) => (b.disabled = false));
    go.textContent = "Try again";
  }
}

export function initUpdates() {
  setTimeout(() => void checkAndDownload().then(maybePrompt), FIRST_CHECK_MS);
  setInterval(() => void checkAndDownload(), RECHECK_MS);
  setInterval(maybePrompt, IDLE_POLL_MS);
}
