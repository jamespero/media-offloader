<script lang="ts">
  import Heading from "./Heading.svelte";
  import Percent from "./Percent.svelte";
  import Icon from "../Icon/Icon.svelte";
  import Row from "../Row/Row.svelte";
  import Button from "../Button/Button.svelte";
  import { plural } from "../../format";
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

<Heading text={heading} />
<PathLine path={root} />
<Percent result value={failed > 0 ? plural(failed, "file") : fallback} note={failed > 0 ? "failed verification" : fallbackNote} />
<Row above="md">
  <Button variant="onStatus" size="sm" onclick={onlog}><Icon name="file" size={13} /> Show log</Button>
</Row>
