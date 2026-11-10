<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import Button from "../Button/Button.svelte";
  import { removeBackup } from "../../destinations";
  import { confirmStop, openRunLog } from "../../dialogs";
  import { destMenu } from "../../menus";
  import { bakState, rootFor } from "../../panelState";
  import { S, hasBackup, running, sep, totalBytes } from "../../state.svelte.ts";
  import DoneStatus from "../Panel/DoneStatus.svelte";
  import LiveStatus from "../Panel/LiveStatus.svelte";
  import Panel from "../Panel/Panel.svelte";
  import PathLine from "../Panel/PathLine.svelte";
  import PreviewLine from "../Panel/PreviewLine.svelte";
  import ProblemStatus from "../Panel/ProblemStatus.svelte";
  import ResumeStatus from "../Panel/ResumeStatus.svelte";
  import ProgressFill from "../ProgressFill/ProgressFill.svelte";
  import TokenChips from "../TokenChips/TokenChips.svelte";

  const panel = $derived(bakState());
  const root = $derived(rootFor("bak") ?? "");
  const resume = $derived(panel === "idle" && !!S.resumeJournal);
  const unset = $derived(!resume && (!hasBackup() || (panel === "idle" && running())));
  const chosen = $derived(!resume && !unset && panel === "idle");
  const dim = $derived(resume ? !hasBackup() : unset ? !S.src || running() : chosen && !S.src);
  const dest = $derived(S.settings.bakDest ?? "");

  const chooseDest = (e: MouseEvent) => destMenu(e.currentTarget as Element, "bak");
</script>

{#snippet behind()}
  {#if chosen && S.src}
    <button class="corner" onclick={removeBackup} title="Remove backup destination"><Icon name="x" size={14} /></button>
  {:else if !resume && !unset && panel === "running"}
    <ProgressFill pct={S.bakP} />
    <span class="tagline">Backup</span>
    <button class="corner" onclick={confirmStop} title="Stop"><Icon name="x" /></button>
  {/if}
{/snippet}

<Panel
  id="pBak"
  {dim}
  status={!resume && !unset && (panel === "running" || panel === "error" || panel === "done") ? panel : ""}
  {behind}
>
  {#if resume}
    {#if hasBackup()}
      <ResumeStatus name="backup" dest={1} />
    {:else}
      <h2>backup</h2>
      <div class="sub">No backup in this offload.</div>
    {/if}
  {:else if unset}
    <h2>backup</h2>
    <div class="sub">Select a location to save a second copy of all media files.</div>
    <Button size="sm" onclick={chooseDest}><Icon name="folder" size={14} /> Choose…</Button>
  {:else if panel === "idle"}
    <h2>backup</h2>
    <div class="sub">Media files will also be copied to:</div>
    <Button size="sm" onclick={chooseDest}><Icon name="folder" size={14} /> Choose…</Button>
    <PathLine path={dest + (dest.endsWith(sep) ? "" : sep)} />
    <TokenChips sameAsCopy />
    <PreviewLine which="bak" />
  {:else if panel === "queued"}
    <h2>backup</h2>
    <PathLine path={root} />
    <StatusPill tone="queued">Queued</StatusPill>
    <div class="sub">Starts as soon as the copy is verified.</div>
  {:else if panel === "running"}
    <LiveStatus
      heading="backing up to"
      {root}
      pct={S.bakP}
      cur={S.runFanOut ? S.cur : S.curB}
      rate={S.runFanOut ? "Reading the card once, writing every drive" : "Reading from the verified copy"}
    />
  {:else if panel === "hold"}
    <h2>backup</h2>
    <PathLine path={root} />
    <StatusPill tone="held">On hold</StatusPill>
    <div class="sub">Resolve the copy errors first.</div>
  {:else if panel === "error"}
    <ProblemStatus heading="backup incomplete" {root} failed={S.failBak} fallback="stopped" fallbackNote={S.abortReason ?? ""} onlog={openRunLog} />
  {:else}
    <DoneStatus heading="backed up to" {root} bytes={totalBytes()} files={S.files.length} verb="backed up" onlog={openRunLog} />
  {/if}
</Panel>
