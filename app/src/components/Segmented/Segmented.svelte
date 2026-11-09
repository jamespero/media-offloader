<script lang="ts">
  interface SegmentItem {
    label: string;
    on: boolean;
    /** A small count after the label. */
    count?: number;
    [attr: string]: unknown;
  }

  let { items, compact = false }: { items: SegmentItem[]; compact?: boolean } = $props();
</script>

<div class={["seg", compact && "compact"]}>
  {#each items as { label, on, count, ...attrs } (label)}
    <button class={[on && "on"]} {...attrs}>{label}{#if count !== undefined}{" "}<span class="n">{count}</span>{/if}</button>
  {/each}
</div>

<style src="./Segmented.css"></style>
