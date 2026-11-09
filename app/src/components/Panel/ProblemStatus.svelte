<script lang="ts">
  import { plural } from "../../format";
  import { ic } from "../../icons";
  import PathLine from "./PathLine.svelte";

  let {
    heading,
    root,
    failed,
    fallback,
    fallbackNote,
    logAct,
  }: {
    heading: string;
    root: string;
    failed: number;
    /** Shown large when no files failed. */
    fallback: string;
    fallbackNote: string;
    logAct: "log" | "histlog";
  } = $props();
</script>

<h2>{heading}</h2>
<PathLine path={root} />
<div class="pct result">
  {#if failed > 0}{plural(failed, "file")}<small>failed verification</small>{:else}{fallback}<small>{fallbackNote}</small>{/if}
</div>
<div class="row result-actions">
  <button class="btn sm" data-act={logAct}>{@html ic.file(13)} Show log</button>
</div>
