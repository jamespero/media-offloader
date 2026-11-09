<script lang="ts">
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import Button from "../Button/Button.svelte";
  import type { HistoryItem } from "../../api";
  import { plural } from "../../format";
  import { clock, day, resumeLabel } from "../../history";
  import { ic } from "../../icons";
  import { running } from "../../state.svelte.ts";
  import Panel from "../Panel/Panel.svelte";

  let { item }: { item: HistoryItem } = $props();

  const outcome = $derived(item.outcome);
</script>

<Panel id="pSrc" source>
  <div class="srcicon">{@html item.sourceKind === "folder" ? ic.folder() : ic.card()}</div>
  <h2>source</h2>
  <div class="vol static"><span>{item.label}</span></div>
  <div class="meta">{day(item.started)} · {clock(item.started)}</div>
  {#if !outcome}
    <StatusPill tone="problem">{@html ic.alert(13)} Didn't finish</StatusPill>
  {:else if outcome.kind === "verified"}
    <StatusPill tone="done">{@html ic.check(13)} Copied &amp; verified</StatusPill>
  {:else if outcome.kind === "failed"}
    <StatusPill tone="problem">{@html ic.alert(13)} {plural(outcome.files, "file")} failed</StatusPill>
  {:else if outcome.kind === "cancelled"}
    <StatusPill tone="queued">Stopped</StatusPill>
  {:else}
    <StatusPill tone="problem">{@html ic.alert(13)} Stopped</StatusPill>
  {/if}
  {#if item.resumable}
    {#if item.sourcePresent}
      <Button variant="primary" start data-act="resume" data-path={item.path} disabled={running()}>{resumeLabel(item)}</Button>
    {:else}
      <div class="sub">Connect {item.label} to {resumeLabel(item).toLowerCase()}.</div>
    {/if}
  {/if}
</Panel>
