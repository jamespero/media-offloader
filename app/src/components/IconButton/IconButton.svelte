<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  let { small = false, children, ...attrs }: Omit<HTMLButtonAttributes, "class" | "children"> & { small?: boolean; children: Snippet } = $props();
</script>

<button class={["iconbtn", small && "sm"]} {...attrs}>{@render children()}</button>

<style src="./IconButton.css"></style>
