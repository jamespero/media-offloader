<script lang="ts">
  import Heading from "../Panel/Heading.svelte";
  import Note from "../Panel/Note.svelte";
  import Percent from "../Panel/Percent.svelte";
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import type { HistoryItem } from "../../api";
  import { openPastLog } from "../../dialogs";
  import DoneStatus from "../Panel/DoneStatus.svelte";
  import Panel from "../Panel/Panel.svelte";
  import PathLine from "../Panel/PathLine.svelte";
  import ProblemStatus from "../Panel/ProblemStatus.svelte";

  /** `dest` is 0 for the copy, 1 for the backup. */
  let { id, item, dest }: { id: string; item: HistoryItem; dest: number } = $props();

  const backup = $derived(dest > 0);
  const root = $derived(item.roots[dest]);
  const done = $derived(item.verified[dest] ?? 0);
  const pct = $derived(item.files ? Math.floor((done / item.files) * 100) : 0);
  const stopped = $derived(!item.outcome || item.outcome.kind === "cancelled");
  const finished = $derived(done === item.files);
</script>

<Panel {id} dim={root === undefined} status={root === undefined ? "" : finished ? "done" : stopped ? "" : "error"}>
  {#if root === undefined}
    <Heading text="backup" />
    <Note>No backup in this offload.</Note>
  {:else if finished}
    <DoneStatus heading={backup ? "backed up to" : "copied to"} {root} bytes={item.bytes} files={item.files} verb={backup ? "backed up" : "copied"} onlog={openPastLog} />
  {:else if stopped}
    <Heading text={backup ? "backup" : "copy"} />
    <PathLine path={root} />
    <Percent result value="{pct}%" note="{done} of {item.files} files verified" />
    <StatusPill tone={item.outcome ? "queued" : "held"}>{item.outcome ? "Stopped" : "Didn't finish"}</StatusPill>
  {:else}
    <ProblemStatus
      heading={backup ? "backup incomplete" : "copy incomplete"}
      {root}
      failed={item.failed[dest] ?? 0}
      fallback="{pct}%"
      fallbackNote="{done} of {item.files} files verified"
      onlog={openPastLog}
    />
  {/if}
</Panel>
