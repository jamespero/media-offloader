<script lang="ts">
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();
</script>

<span class="mono">{@render children()}</span>

<style src="./Mono.css"></style>
