<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../../api";
  import { ic, type IconName } from "../../icons";
  import { S, running } from "../../state.svelte.ts";
  import NamingPane from "./NamingPane.svelte";
  import SettingRow from "./SettingRow.svelte";
  import { settingsTab } from "./settingsState.svelte.ts";
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

  let logDir = $state("");

  onMount(() => {
    api.logDir().then((d) => (logDir = d ?? ""));
  });
</script>

{#snippet segment(act: string, on: boolean, off: string, onLabel: string)}
  <div class="seg">
    <button class={[!on && "on"]} data-act={act} data-v="0" disabled={busy}>{off}</button>
    <button class={[on && "on"]} data-act={act} data-v="1" disabled={busy}>{onLabel}</button>
  </div>
{/snippet}

<div class="modal">
  <div class="nav">
    <h4>settings</h4>
    {#each TABS as [key, label, icon] (key)}
      <button class={["mi", key === tab && "on"]} data-act="stab" data-tab={key}>{@html ic[icon](15)}<span>{label}</span></button>
    {/each}
  </div>
  <div class="pane">
    <button class="iconbtn closex" data-act="closeov" aria-label="Close">{@html ic.x()}</button>
    {#if tab === "general"}
      <h5>general</h5>
      <p class="intro">How Media Offloader reacts when media shows up.</p>
      <ToggleRow title="Auto-select inserted media" note="When a card or drive is connected, make it the source. Folders are never auto-selected." setting="autoSelect" />
      <ToggleRow title="Copy hidden system files" note="Include .Trashes, .Spotlight-V100, System Volume Information. Off still copies every camera file and folder." setting="includeSystemFiles" />
      <ToggleRow title="Eject source when verified" note="Unmount the card as soon as its copy is verified, so it is safe to pull. The backup carries on from the copy." setting="ejectWhenVerified" />
      <ToggleRow title="Keep the computer awake" note="Prevent sleep while a transfer is running." setting="keepAwake" />
      <ToggleRow title="Install updates automatically" note="When a new version is out, install it as the app starts, before you begin. Off: the app asks first. Never during an offload." setting="autoUpdate" />
    {:else if tab === "verify"}
      <h5>verification</h5>
      <p class="intro">Every file is read back from the destination and compared to the source.</p>
      <SettingRow title="Checksum" note="xxHash64 is fast and is what ASC MHL uses. Add MD5 when production or insurance asks for it.">
        {@render segment("md5", s.md5, "xxHash64", "xxHash64 + MD5")}
      </SettingRow>
      <ToggleRow title="Write ASC MHL manifest" note="Saves an ascmhl folder next to the footage so any copy can be re-verified later, here or in any MHL tool." setting="mhl" />
      <SettingRow title="Write .md5 checksum list" note={'Classic "hash  path" file (checksums.md5) next to the footage. Requires MD5.'}>
        <Toggle on={s.md5List && s.md5} setting="md5List" label="Write .md5 checksum list" lock={!s.md5} />
      </SettingRow>
      <SettingRow title="Verify from disk, not cache" note="Reads bypass the OS cache so verification checks what is actually on the drive. Always on.">
        <Toggle on label="Verify from disk, not cache" lock />
      </SettingRow>
    {:else if tab === "transfers"}
      <h5>transfers</h5>
      <p class="intro">How copy and backup are scheduled.</p>
      <SettingRow title="Copy mode" note="Sequential: copy, verify, free the card, then back up from the copy (classic). Fan-out: read the card once and write every drive at the same time.">
        {@render segment("fan", s.fanOut, "Sequential", "Fan-out")}
      </SettingRow>
      <SettingRow title="When the destination already exists" note="Never overwrite. Repeat offloads go into copy-conflicted, copy-conflicted-1, …">
        <span class="meta">Always on</span>
      </SettingRow>
    {:else if tab === "naming"}
      <NamingPane />
    {:else if tab === "notify"}
      <h5>notifications</h5>
      <p class="intro">Know it's done without watching the screen.</p>
      <ToggleRow title="Play a sound when finished" note="A rising chime when everything verified, a lower one when something needs attention." setting="sound" />
      <ToggleRow title="System notification" note="Shows in Notification Center / the Windows notification area when the app isn't in front." setting="notify" />
    {:else}
      <h5>logs</h5>
      <p class="intro">A plain-text log is kept for every offload: each file's checksums, what was skipped or renamed, and how it ended.</p>
      <SettingRow title="Log location" note={logDir} mono>
        <button class="btn sm" data-act="logsfolder">{@html ic.reveal(13)} Show</button>
      </SettingRow>
    {/if}
  </div>
</div>
