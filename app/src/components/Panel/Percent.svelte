<script lang="ts">
  /** A panel's big number with a caption under it. `result` is a little smaller, for an outcome. */
  let { value, note, result = false }: { value: string; note: string; result?: boolean } = $props();
</script>

<div class={["pct", { result }]}>{value}<small>{note}</small></div>

<style src="./Percent.css"></style>
