<script lang="ts">
  import MenuHeading from "../Menu/MenuHeading.svelte";
  import MenuNote from "../Menu/MenuNote.svelte";
  import MenuSeparator from "../Menu/MenuSeparator.svelte";
  import { settings } from "../Settings/settingsState.svelte.ts";
  import { savePresetDialog } from "../../dialogs";
  import { activePreset, usePreset } from "../../presets";
  import { S, running } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  const active = $derived(activePreset());
</script>

<MenuHeading text={"Presets"} />
{#each S.settings.presets as p (p.id)}
  <MenuItem icon="star" label={p.name} onclick={() => usePreset(p.id)} disabled={running()} checked={active?.id === p.id} />
{:else}
  <MenuNote>No presets yet. Set up the subfolders for a camera, then save them here.</MenuNote>
{/each}
<MenuSeparator />
<MenuItem icon="plus" label="Save Current as Preset…" onclick={savePresetDialog} disabled={!S.settings.tokens.length} />
<MenuItem icon="gear" label="Manage Presets…" onclick={() => settings("naming")} />
