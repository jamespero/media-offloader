<script lang="ts">
  import { ic } from "../../icons";
  import { S, sep, tokenContext } from "../../state.svelte.ts";
  import { tokenText } from "../../tokens";
  import SettingRow from "./SettingRow.svelte";

  const ctx = $derived(tokenContext());

  function folders(tokens: (typeof S.settings.tokens)) {
    const path = tokens.map((t) => tokenText(t, ctx)).join(sep);
    return path ? `…${sep}${path}` : "no subfolders";
  }
</script>

<h5>naming &amp; presets</h5>
<p class="intro">A preset saves a subfolder pattern, e.g. one per camera. It goes under the copy and backup destinations, which you set once per project. Pick a preset from the ★ Preset button above the thumbnails.</p>
{#each S.settings.presets as p (p.id)}
  <SettingRow title={p.name} note={folders(p.tokens)} mono>
    <button class="btn sm" data-act="delpreset" data-id={p.id} title="Delete preset">{@html ic.trash(13)}</button>
  </SettingRow>
{:else}
  <div class="set"><div class="l"><span>No presets yet. Use ★ Preset → Save Current as Preset….</span></div></div>
{/each}
<SettingRow title="Import # counter" note="The next import number. Counts up after each completed offload.">
  <div class="row tight">
    <button class="btn sm" data-act="importdec" aria-label="Lower">−</button>
    <span class="meta mono stepper-value">{String(S.settings.importNo).padStart(3, "0")}</span>
    <button class="btn sm" data-act="importinc" aria-label="Raise">+</button>
  </div>
</SettingRow>
