<script lang="ts">
  import { settings } from "../Settings/settingsState.svelte.ts";
  import { savePresetDialog } from "../../dialogs";
  import { activePreset, usePreset } from "../../presets";
  import { S, running } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  const active = $derived(activePreset());
</script>

<div class="mh">Presets</div>
{#each S.settings.presets as p (p.id)}
  <MenuItem icon="star" label={p.name} onclick={() => usePreset(p.id)} disabled={running()} checked={active?.id === p.id} />
{:else}
  <div class="mnote">No presets yet. Set up the subfolders for a camera, then save them here.</div>
{/each}
<div class="sep"></div>
<MenuItem icon="plus" label="Save Current as Preset…" onclick={savePresetDialog} disabled={!S.settings.tokens.length} />
<MenuItem icon="gear" label="Manage Presets…" onclick={() => settings("naming")} />
