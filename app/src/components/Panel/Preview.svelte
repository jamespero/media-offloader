<script lang="ts">
  /** A small line under a destination: `prefix`, then `strong` in bold, then `suffix`; or a warning. */
  let {
    title,
    prefix = "",
    strong = "",
    suffix = "",
    warn,
  }: { title?: string; prefix?: string; strong?: string; suffix?: string; warn?: string } = $props();
</script>

<div class="preview" {title}>
  {#if warn}<span class="warn">{warn}</span>{:else}{prefix}<b>{strong}</b>{suffix}{/if}
</div>

<style src="./Preview.css"></style>
