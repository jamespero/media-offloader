<script lang="ts">
  import type { Snippet } from "svelte";

  /** A line of explanation in a panel. `strong` for one that should stand out. */
  let { strong = false, children }: { strong?: boolean; children: Snippet } = $props();
</script>

<p class={["note", { strong }]}>{@render children()}</p>

<style src="./Note.css"></style>
