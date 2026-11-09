<script lang="ts">
  import { fmtSize } from "../../format";
  import { S } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";
</script>

<div class="mh">Connected media</div>
{#each S.volumes as v (v.path)}
  <MenuItem icon="disk" label={v.label} act="pick" data-path={v.path} title={v.path} hint={fmtSize(v.total)} />
{:else}
  <div class="mnote">No cards or drives found.</div>
{/each}
<div class="sep"></div>
<MenuItem icon="folder" label="Choose Folder…" act="pickfolder" />
