<script lang="ts">
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();
</script>

<div class="dialog">{@render children()}</div>

<style src="./Dialog.css"></style>
