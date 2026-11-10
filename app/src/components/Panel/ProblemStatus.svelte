<script lang="ts">
  import Row from "../Row/Row.svelte";
  import Button from "../Button/Button.svelte";
  import { plural } from "../../format";
  import { ic } from "../../icons";
  import PathLine from "./PathLine.svelte";

  let {
    heading,
    root,
    failed,
    fallback,
    fallbackNote,
    onlog,
  }: {
    heading: string;
    root: string;
    failed: number;
    /** Shown large when no files failed. */
    fallback: string;
    fallbackNote: string;
    onlog: () => void;
  } = $props();
</script>

<h2>{heading}</h2>
<PathLine path={root} />
<div class="pct result">
  {#if failed > 0}{plural(failed, "file")}<small>failed verification</small>{:else}{fallback}<small>{fallbackNote}</small>{/if}
</div>
<Row above="md">
  <Button variant="onStatus" size="sm" onclick={onlog}>{@html ic.file(13)} Show log</Button>
</Row>
