<script lang="ts">
  import { S } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  let { which }: { which: "copy" | "bak" } = $props();

  const recent = $derived((which === "copy" ? S.settings.recentCopy : S.settings.recentBak).slice(0, 5));
</script>

{#if recent.length}
  <div class="mh">Recent</div>
  {#each recent as path (path)}
    <MenuItem icon="folder" label={path} mono act="setdest" data-w={which} data-v={path} />
  {/each}
  <div class="sep"></div>
{/if}
<MenuItem icon="reveal" label="Browse…" act="browsedest" data-w={which} />
