<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, note, mono = false, children }: { title: string; note: string; mono?: boolean; children: Snippet } = $props();
</script>

<div class="set">
  <div class="l">
    <b>{title}</b>
    <span>{#if mono}<span class="mono">{note}</span>{:else}{note}{/if}</span>
  </div>
  {@render children()}
</div>
