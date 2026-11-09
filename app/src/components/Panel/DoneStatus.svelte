<script lang="ts">
  import { fmtSize, plural } from "../../format";
  import { ic } from "../../icons";
  import PathLine from "./PathLine.svelte";

  let {
    heading,
    root,
    bytes,
    files,
    verb,
    logAct,
  }: { heading: string; root: string; bytes: number; files: number; verb: string; logAct: "log" | "histlog" } = $props();
</script>

<h2>{heading}</h2>
<PathLine path={root} />
<div class="pct">100%<small>complete</small></div>
<div class="okring">{@html ic.check(20, 2.6)}</div>
<button class="summary" data-act={logAct} title="Open the log for this offload">
  {fmtSize(bytes)} in {plural(files, "file")} · {verb} &amp; checksum verified {@html ic.info(12)}
</button>
