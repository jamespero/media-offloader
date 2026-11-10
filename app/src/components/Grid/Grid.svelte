<script lang="ts">
  import { gridKind, kindOf } from "../../fileKinds";
  import type { FileView } from "../../state.svelte.ts";
  import Thumb from "./Thumb.svelte";

  let { files }: { files: FileView[] } = $props();

  const shown = $derived(gridKind(files));
</script>

<!-- The tiles of the shown kind, plus any that failed. -->
<div class="grid">
  {#each files as file}
    {#if shown === "all" || kindOf(file.ext) === shown || file.state === "failed"}
      <Thumb {file} />
    {/if}
  {/each}
</div>

<style src="./Grid.css"></style>
