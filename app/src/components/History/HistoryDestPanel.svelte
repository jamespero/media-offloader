<script lang="ts">
  import type { HistoryItem } from "../../api";
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
    <h2>backup</h2>
    <div class="sub">No backup in this offload.</div>
  {:else if finished}
    <DoneStatus heading={backup ? "backed up to" : "copied to"} {root} bytes={item.bytes} files={item.files} verb={backup ? "backed up" : "copied"} logAct="histlog" />
  {:else if stopped}
    <h2>{backup ? "backup" : "copy"}</h2>
    <PathLine path={root} />
    <div class="pct result">{pct}%<small>{done} of {item.files} files verified</small></div>
    <span class="qpill"><i class={[!item.outcome && "problem"]}></i>{item.outcome ? "Stopped" : "Didn't finish"}</span>
  {:else}
    <ProblemStatus
      heading={backup ? "backup incomplete" : "copy incomplete"}
      {root}
      failed={item.failed[dest] ?? 0}
      fallback="{pct}%"
      fallbackNote="{done} of {item.files} files verified"
      logAct="histlog"
    />
  {/if}
</Panel>
