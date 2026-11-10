<script lang="ts">
  import Heading from "./Heading.svelte";
  import Percent from "./Percent.svelte";
  import type { Live } from "../../state.svelte.ts";
  import PathLine from "./PathLine.svelte";

  let { heading, root, pct, cur, rate }: { heading: string; root: string; pct: number; cur: Live | null; rate: string } = $props();
</script>

<Heading text={heading} />
<PathLine path={root} />
<Percent value="{Math.floor(pct)}%" note="complete" />
<div class="status">
  {#if cur}<span>{cur.verify ? "Verifying checksum" : "Copying"}</span><span class="f">{cur.name}</span>{/if}
</div>
<div class="rate">{rate}</div>

<style src="./LiveStatus.css"></style>
