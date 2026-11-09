<script lang="ts">
  import type { Live } from "../../state.svelte.ts";
  import PathLine from "./PathLine.svelte";

  let { heading, root, pct, cur, rate }: { heading: string; root: string; pct: number; cur: Live | null; rate: string } = $props();
</script>

<h2>{heading}</h2>
<PathLine path={root} />
<div class="pct"><span class="n">{Math.floor(pct)}%</span><small>complete</small></div>
<div class="status">
  {#if cur}<span>{cur.verify ? "Verifying checksum" : "Copying"}</span><span class="f">{cur.name}</span>{/if}
</div>
<div class="rate">{rate}</div>
