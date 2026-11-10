<script lang="ts">
  import Heading from "./Heading.svelte";
  import Percent from "./Percent.svelte";
  import Icon from "../Icon/Icon.svelte";
  import { fmtSize, plural } from "../../format";
  import PathLine from "./PathLine.svelte";

  let {
    heading,
    root,
    bytes,
    files,
    verb,
    onlog,
  }: { heading: string; root: string; bytes: number; files: number; verb: string; onlog: () => void } = $props();
</script>

<Heading text={heading} />
<PathLine path={root} />
<Percent value="100%" note="complete" />
<div class="okring"><Icon name="check" size={20} stroke={2.6} /></div>
<button class="summary" onclick={onlog} title="Open the log for this offload">
  {fmtSize(bytes)} in {plural(files, "file")} · {verb} &amp; checksum verified <Icon name="info" size={12} />
</button>

<style src="./DoneStatus.css"></style>
