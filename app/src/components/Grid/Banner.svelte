<script lang="ts">
  import type { Snippet } from "svelte";

  let { icon, tone = "", action, children }: { icon: string; tone?: "" | "info" | "bad"; action?: Snippet; children: Snippet } = $props();
</script>

<div class="banner">
  <div class={["ic", tone]}>{@html icon}</div>
  <div class="tx">{@render children()}</div>
  {@render action?.()}
</div>
