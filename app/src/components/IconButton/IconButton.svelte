<script lang="ts">
  import type { Snippet } from "svelte";

  let { small = false, children, ...attrs }: { small?: boolean; children: Snippet; [attr: string]: unknown } = $props();
</script>

<button class={["iconbtn", small && "sm"]} {...attrs}>{@render children()}</button>

<style src="./IconButton.css"></style>
