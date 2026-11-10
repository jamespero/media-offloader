<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import Segmented from "../Segmented/Segmented.svelte";
  import MenuItem from "../Menu/MenuItem.svelte";
  import IconButton from "../IconButton/IconButton.svelte";
  import Button from "../Button/Button.svelte";
  import { onMount } from "svelte";
  import { api } from "../../api";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import type { IconName } from "../../icons";
  import { S, running } from "../../state.svelte.ts";
  import { showLogsFolder } from "../../places";
  import { ejectEnabled, rereadSource } from "../../source";
  import NamingPane from "./NamingPane.svelte";
  import SettingRow from "./SettingRow.svelte";
  import { settings, settingsTab } from "./settingsState.svelte.ts";
  import Toggle from "./Toggle.svelte";
  import ToggleRow from "./ToggleRow.svelte";

  const TABS: [string, string, IconName][] = [
    ["general", "General", "gear"],
    ["verify", "Verification", "shield"],
    ["transfers", "Transfers", "layers"],
    ["naming", "Naming & Presets", "star"],
    ["notify", "Notifications", "bell"],
    ["logs", "Logs", "file"],
  ];

  const s = S.settings;
  const tab = $derived(settingsTab());
  const busy = $derived(running());

  /** The items of a setting with two choices; the second is `on`. */
  const choices = (set: (on: boolean) => void, on: boolean, off: string, onLabel: string) => [
    { label: off, on: !on, onclick: () => set(false), disabled: busy },
    { label: onLabel, on, onclick: () => set(true), disabled: busy },
  ];

  let logDir = $state("");

  onMount(() => {
    api.logDir().then((d) => (logDir = d ?? ""));
  });
</script>

<div class="modal">
  <div class="nav">
    <h4>settings</h4>
    {#each TABS as [key, label, icon] (key)}
      <MenuItem {icon} {label} onclick={() => settings(key)} on={key === tab} tall />
    {/each}
  </div>
  <div class="pane">
    <div class="closex"><IconButton onclick={closeOverlay} aria-label="Close"><Icon name="x" /></IconButton></div>
    {#if tab === "general"}
      <h5>general</h5>
      <p class="intro">How Media Offloader reacts when media shows up.</p>
      <ToggleRow title="Auto-select inserted media" note="When a card or drive is connected, make it the source. Folders are never auto-selected." bind:on={s.autoSelect} />
      <ToggleRow title="Copy hidden system files" note="Include .Trashes, .Spotlight-V100, System Volume Information. Off still copies every camera file and folder." bind:on={s.includeSystemFiles} onchange={rereadSource} />
      {#if ejectEnabled}
        <ToggleRow title="Eject source when verified" note="Unmount the card as soon as its copy is verified, so it is safe to pull. The backup carries on from the copy." bind:on={s.ejectWhenVerified} />
      {/if}
      <ToggleRow title="Keep the computer awake" note="Prevent sleep while a transfer is running." bind:on={s.keepAwake} />
      <ToggleRow title="Install updates automatically" note="When a new version is out, install it as the app starts, before you begin. Off: the app asks first. Never during an offload." bind:on={s.autoUpdate} />
    {:else if tab === "verify"}
      <h5>verification</h5>
      <p class="intro">Every file is read back from the destination and compared to the source.</p>
      <SettingRow title="Checksum" note="xxHash64 is fast and is what ASC MHL uses. Add MD5 when production or insurance asks for it.">
        <Segmented items={choices((on) => (s.md5 = on), s.md5, "xxHash64", "xxHash64 + MD5")} />
      </SettingRow>
      <ToggleRow title="Write ASC MHL manifest" note="Saves an ascmhl folder next to the footage so any copy can be re-verified later, here or in any MHL tool." bind:on={s.mhl} />
      <SettingRow title="Write .md5 checksum list" note={'Classic "hash  path" file (checksums.md5) next to the footage. Requires MD5.'}>
        {#if s.md5}
          <Toggle bind:on={s.md5List} label="Write .md5 checksum list" />
        {:else}
          <Toggle label="Write .md5 checksum list" lock />
        {/if}
      </SettingRow>
      <SettingRow title="Verify from disk, not cache" note="Reads bypass the OS cache so verification checks what is actually on the drive. Always on.">
        <Toggle on label="Verify from disk, not cache" lock />
      </SettingRow>
    {:else if tab === "transfers"}
      <h5>transfers</h5>
      <p class="intro">How copy and backup are scheduled.</p>
      <SettingRow title="Copy mode" note="Sequential: copy, verify, free the card, then back up from the copy (classic). Fan-out: read the card once and write every drive at the same time.">
        <Segmented items={choices((on) => (s.fanOut = on), s.fanOut, "Sequential", "Fan-out")} />
      </SettingRow>
      <SettingRow title="When the destination already exists" note="Never overwrite. Repeat offloads go into copy-conflicted, copy-conflicted-1, …">
        <span class="always">Always on</span>
      </SettingRow>
    {:else if tab === "naming"}
      <NamingPane />
    {:else if tab === "notify"}
      <h5>notifications</h5>
      <p class="intro">Know it's done without watching the screen.</p>
      <ToggleRow title="Play a sound when finished" note="A rising chime when everything verified, a lower one when something needs attention." bind:on={s.sound} />
      <ToggleRow title="System notification" note="Shows in Notification Center / the Windows notification area when the app isn't in front." bind:on={s.notify} />
    {:else}
      <h5>logs</h5>
      <p class="intro">A plain-text log is kept for every offload: each file's checksums, what was skipped or renamed, and how it ended.</p>
      <SettingRow title="Log location" note={logDir} mono>
        <Button size="sm" onclick={showLogsFolder}><Icon name="reveal" size={13} /> Show</Button>
      </SettingRow>
    {/if}
  </div>
</div>

<style src="./Settings.css"></style>
