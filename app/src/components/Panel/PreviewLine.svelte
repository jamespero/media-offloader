<script lang="ts">
  import { rootFor } from "../../panelState";
  import { S, sep } from "../../state.svelte.ts";

  let { which }: { which: "copy" | "bak" } = $props();

  const root = $derived(rootFor(which));
</script>

{#if S.prepError && which === "copy"}
  <div class="preview"><span class="warn">{S.prepError}</span></div>
{:else if root && S.src}
  <div class="preview" title={root}>→ <b>{root}{sep}</b></div>
{/if}
