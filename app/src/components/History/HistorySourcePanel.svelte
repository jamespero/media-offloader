<script lang="ts">
  import Note from "../Panel/Note.svelte";
  import SourceHead from "../Panel/SourceHead.svelte";
  import Icon from "../Icon/Icon.svelte";
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import Button from "../Button/Button.svelte";
  import type { HistoryItem } from "../../api";
  import { plural } from "../../format";
  import { clock, day, resumeLabel } from "../../history";
  import { resumeFrom } from "../../source";
  import { running } from "../../state.svelte.ts";
  import Panel from "../Panel/Panel.svelte";

  let { item }: { item: HistoryItem } = $props();

  const outcome = $derived(item.outcome);
</script>

<Panel id="pSrc" source>
  <SourceHead still icon={item.sourceKind === "folder" ? "folder" : "card"} label={item.label} meta="{day(item.started)} · {clock(item.started)}" />
  {#if !outcome}
    <StatusPill tone="problem"><Icon name="alert" size={13} /> Didn't finish</StatusPill>
  {:else if outcome.kind === "verified"}
    <StatusPill tone="done"><Icon name="check" size={13} /> Copied &amp; verified</StatusPill>
  {:else if outcome.kind === "failed"}
    <StatusPill tone="problem"><Icon name="alert" size={13} /> {plural(outcome.files, "file")} failed</StatusPill>
  {:else if outcome.kind === "cancelled"}
    <StatusPill tone="queued">Stopped</StatusPill>
  {:else}
    <StatusPill tone="problem"><Icon name="alert" size={13} /> Stopped</StatusPill>
  {/if}
  {#if item.resumable}
    {#if item.sourcePresent}
      <Button variant="primary" start onclick={() => resumeFrom(item.path)} disabled={running()}>{resumeLabel(item)}</Button>
    {:else}
      <Note>Connect {item.label} to {resumeLabel(item).toLowerCase()}.</Note>
    {/if}
  {/if}
</Panel>
