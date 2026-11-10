<script lang="ts">
  import MenuHeading from "../Menu/MenuHeading.svelte";
  import MenuSeparator from "../Menu/MenuSeparator.svelte";
  import { browseDest, setDest } from "../../destinations";
  import { S } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  let { which }: { which: "copy" | "bak" } = $props();

  const recent = $derived((which === "copy" ? S.settings.recentCopy : S.settings.recentBak).slice(0, 5));
</script>

{#if recent.length}
  <MenuHeading text={"Recent"} />
  {#each recent as path (path)}
    <MenuItem icon="folder" label={path} mono onclick={() => setDest(which, path)} />
  {/each}
  <MenuSeparator />
{/if}
<MenuItem icon="reveal" label="Browse…" onclick={() => browseDest(which)} />
