<script lang="ts">
  import Heading from "../Panel/Heading.svelte";
  import Note from "../Panel/Note.svelte";
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
  import TokenChips from "../TokenChips/TokenChips.svelte";

  const panel = $derived(bakState());
  const root = $derived(rootFor("bak") ?? "");
  const resume = $derived(panel === "idle" && !!S.resumeJournal);
  const unset = $derived(!resume && (!hasBackup() || (panel === "idle" && running())));
  const chosen = $derived(!resume && !unset && panel === "idle");
  const dim = $derived(resume ? !hasBackup() : unset ? !S.src || running() : chosen && !S.src);
  const dest = $derived(S.settings.bakDest ?? "");
  /** The backup is being written now (not merely queued or held). */
  const writing = $derived(!resume && !unset && panel === "running");

  const chooseDest = (e: MouseEvent) => destMenu(e.currentTarget as Element, "bak");
</script>

<Panel
  id="pBak"
  {dim}
  status={!resume && !unset && (panel === "running" || panel === "error" || panel === "done") ? panel : ""}
  progress={writing ? S.bakP : undefined}
  tagline={writing ? "Backup" : undefined}
  corner={chosen && S.src
    ? { title: "Remove backup destination", onclick: removeBackup, iconSize: 14 }
    : writing
      ? { title: "Stop", onclick: confirmStop }
      : undefined}
>
  {#if resume}
    {#if hasBackup()}
      <ResumeStatus name="backup" dest={1} />
    {:else}
      <Heading text="backup" />
      <Note>No backup in this offload.</Note>
    {/if}
  {:else if unset}
    <Heading text="backup" />
    <Note>Select a location to save a second copy of all media files.</Note>
    <Button size="sm" onclick={chooseDest}><Icon name="folder" size={14} /> Choose…</Button>
  {:else if panel === "idle"}
    <Heading text="backup" />
    <Note>Media files will also be copied to:</Note>
    <Button size="sm" onclick={chooseDest}><Icon name="folder" size={14} /> Choose…</Button>
    <PathLine path={dest + (dest.endsWith(sep) ? "" : sep)} />
    <TokenChips sameAsCopy />
    <PreviewLine which="bak" />
  {:else if panel === "queued"}
    <Heading text="backup" />
    <PathLine path={root} />
    <StatusPill tone="queued">Queued</StatusPill>
    <Note>Starts as soon as the copy is verified.</Note>
  {:else if panel === "running"}
    <LiveStatus
      heading="backing up to"
      {root}
      pct={S.bakP}
      cur={S.runFanOut ? S.cur : S.curB}
      rate={S.runFanOut ? "Reading the card once, writing every drive" : "Reading from the verified copy"}
    />
  {:else if panel === "hold"}
    <Heading text="backup" />
    <PathLine path={root} />
    <StatusPill tone="held">On hold</StatusPill>
    <Note>Resolve the copy errors first.</Note>
  {:else if panel === "error"}
    <ProblemStatus heading="backup incomplete" {root} failed={S.failBak} fallback="stopped" fallbackNote={S.abortReason ?? ""} onlog={openRunLog} />
  {:else}
    <DoneStatus heading="backed up to" {root} bytes={totalBytes()} files={S.files.length} verb="backed up" onlog={openRunLog} />
  {/if}
</Panel>
