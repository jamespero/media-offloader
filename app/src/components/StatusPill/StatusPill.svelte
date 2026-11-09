<script lang="ts">
  import type { Snippet } from "svelte";

  /** `done` and `problem` are colored; `queued` and `held` are neutral, with a dot. */
  let { tone, children }: { tone: "done" | "problem" | "queued" | "held"; children: Snippet } = $props();
</script>

{#if tone === "queued" || tone === "held"}
  <span class="dotted"><i class={[tone === "held" && "held"]}></i>{@render children()}</span>
{:else}
  <span class={tone}>{@render children()}</span>
{/if}

<style src="./StatusPill.css"></style>
