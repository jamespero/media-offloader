<script lang="ts">
  import MenuHeading from "../Menu/MenuHeading.svelte";
  import MenuNote from "../Menu/MenuNote.svelte";
  import MenuSeparator from "../Menu/MenuSeparator.svelte";
  import { fmtSize } from "../../format";
  import { pickFolder, pickVolume } from "../../source";
  import { S } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";
</script>

<MenuHeading text={"Connected media"} />
{#each S.volumes as v (v.path)}
  <MenuItem icon="disk" label={v.label} onclick={() => pickVolume(v.path)} title={v.path} hint={fmtSize(v.total)} />
{:else}
  <MenuNote>No cards or drives found.</MenuNote>
{/each}
<MenuSeparator />
<MenuItem icon="folder" label="Choose Folder…" onclick={pickFolder} />
