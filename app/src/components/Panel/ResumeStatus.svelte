<script lang="ts">
  import Heading from "./Heading.svelte";
  import Note from "./Note.svelte";
  import Preview from "./Preview.svelte";
  import { resuming } from "../../panelState";
  import { S } from "../../state.svelte.ts";
  import PathLine from "./PathLine.svelte";

  /** `dest` is 0 for the copy, 1 for the backup. */
  let { name, dest }: { name: "copy" | "backup"; dest: number } = $props();

  const target = $derived(S.dests?.[dest]);
  const item = $derived(resuming());
</script>

<Heading text={name} />
<Note>Continuing in the same folder:</Note>
{#if target}<PathLine path={target.root} />{/if}
<Preview strong="{item?.verified[dest] ?? 0} of {item?.files ?? S.files.length}" suffix=" files already verified" />
