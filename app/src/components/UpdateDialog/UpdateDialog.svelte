<script lang="ts">
  import { relaunch } from "@tauri-apps/plugin-process";
  import type { Update } from "@tauri-apps/plugin-updater";
  import { onMount } from "svelte";
  import { api } from "../../api";
  import { reverifyRunning } from "../../reverify.svelte.ts";
  import { S, running, saveSettings } from "../../state.svelte.ts";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";

  /** With `auto` on, it says what's happening for a moment, then installs by itself. */
  let { update, auto = false }: { update: Update; auto?: boolean } = $props();

  const AUTO_DELAY_MS = 2500;

  let installing = $state(false);
  let error = $state<string | null>(null);

  async function install() {
    // The dialog only shows when idle and covers the window, but check once more:
    // restarting mid-offload is the one thing this must never do.
    if (running() || reverifyRunning()) return;
    installing = true;
    error = null;
    try {
      await update.install(); // on Windows the installer closes the app and reopens it itself
      await relaunch();
    } catch (err) {
      error = `The update didn't install (${err}). Try again, or download it from the releases page.`;
      installing = false;
    }
  }

  function toggleAuto() {
    S.settings.autoUpdate = !S.settings.autoUpdate;
    saveSettings();
  }

  onMount(() => {
    if (!auto) return;
    const timer = setTimeout(install, AUTO_DELAY_MS);
    return () => clearTimeout(timer);
  });
</script>

<div class="dialog">
  {#if auto}
    <h3>Updating</h3>
    <p>Installing Media Offloader {update.version}. The app restarts in a moment.</p>
    {#if error}<p id="updMsg" class="problem-text">{error}</p>{:else}<p id="updMsg" class="note">Automatic updates can be turned off in Settings → General.</p>{/if}
  {:else}
    <h3>Update ready</h3>
    <p>Media Offloader {update.version} has been downloaded. You have {update.currentVersion}.</p>
    {#if error}<p id="updMsg" class="problem-text">{error}</p>{:else}<p id="updMsg">Installing takes a few seconds and restarts the app. Your settings and presets are kept.</p>{/if}
    <div class="set updauto">
      <div class="l">
        <b>Install updates automatically from now on</b>
        <span>As the app starts, before you begin. Never during an offload.</span>
      </div>
      <button class={["tog", S.settings.autoUpdate && "on"]} id="updAuto" aria-pressed={S.settings.autoUpdate} aria-label="Install updates automatically from now on" disabled={installing} onclick={toggleAuto}></button>
    </div>
  {/if}
  <div class="acts">
    {#if !auto}
      <button class="btn" id="updNotes" disabled={installing} onclick={() => api.openRelease(update.version).catch(() => {})}>What's new</button>
    {/if}
    <button class="btn" id="updLater" disabled={installing} onclick={closeOverlay}>Not now</button>
    <button class="btn primary" id="updGo" disabled={installing || (auto && !error)} onclick={install}>
      {#if error}Try again{:else if installing || auto}Installing…{:else}Update and relaunch{/if}
    </button>
  </div>
</div>
