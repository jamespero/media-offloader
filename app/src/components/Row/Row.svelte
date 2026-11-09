<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    gap = "md",
    above = "none",
    children,
  }: {
    gap?: "md" | "tight" | "tighter";
    /** Room above the row. */
    above?: "none" | "sm" | "md";
    children: Snippet;
  } = $props();
</script>

<div class={["row", gap !== "md" && gap, above !== "none" && `above-${above}`]}>{@render children()}</div>

<style src="./Row.css"></style>
