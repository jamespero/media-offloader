<script lang="ts">
  import type { Snippet } from "svelte";

  /** A line of explanation in a menu, such as "No cards or drives found." */
  let { children }: { children: Snippet } = $props();
</script>

<div class="mnote">{@render children()}</div>

<style src="./MenuNote.css"></style>
