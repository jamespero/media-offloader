<script lang="ts">
  import type { Snippet } from "svelte";

  /** A small dialog: a title, its content (usually a paragraph or two), and its buttons. */
  let { title, children, actions }: { title: string; children: Snippet; actions: Snippet } = $props();
</script>

<div class="dialog">
  <h3>{title}</h3>
  {@render children()}
  <div class="acts">{@render actions()}</div>
</div>

<style src="./Dialog.css"></style>
