<script lang="ts">
  import Preview from "./Preview.svelte";
  import { rootFor } from "../../panelState";
  import { S, sep } from "../../state.svelte.ts";

  let { which }: { which: "copy" | "bak" } = $props();

  const root = $derived(rootFor(which));
</script>

{#if S.prepError && which === "copy"}
  <Preview warn={S.prepError} />
{:else if root && S.src}
  <Preview title={root} prefix="→ " strong="{root}{sep}" />
{/if}
