<script lang="ts">
  import { resuming } from "../../panelState";
  import { S } from "../../state.svelte.ts";
  import PathLine from "./PathLine.svelte";

  /** `dest` is 0 for the copy, 1 for the backup. */
  let { name, dest }: { name: "copy" | "backup"; dest: number } = $props();

  const target = $derived(S.dests?.[dest]);
  const item = $derived(resuming());
</script>

<h2>{name}</h2>
<div class="sub">Continuing in the same folder:</div>
{#if target}<PathLine path={target.root} />{/if}
<div class="preview"><b>{item?.verified[dest] ?? 0} of {item?.files ?? S.files.length}</b> files already verified</div>
