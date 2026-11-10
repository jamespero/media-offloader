<script lang="ts">
  import { fmtSize } from "../../format";
  import { pickFolder, pickVolume } from "../../source";
  import { S } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";
</script>

<div class="mh">Connected media</div>
{#each S.volumes as v (v.path)}
  <MenuItem icon="disk" label={v.label} onclick={() => pickVolume(v.path)} title={v.path} hint={fmtSize(v.total)} />
{:else}
  <div class="mnote">No cards or drives found.</div>
{/each}
<div class="sep"></div>
<MenuItem icon="folder" label="Choose Folder…" onclick={pickFolder} />
