<script lang="ts">
  import { ic } from "../../icons";
  import { copyState, rateText, rootFor } from "../../panelState";
  import { S, sep, totalBytes } from "../../state.svelte.ts";
  import DoneStatus from "../Panel/DoneStatus.svelte";
  import LiveStatus from "../Panel/LiveStatus.svelte";
  import Panel from "../Panel/Panel.svelte";
  import PathLine from "../Panel/PathLine.svelte";
  import PreviewLine from "../Panel/PreviewLine.svelte";
  import ProblemStatus from "../Panel/ProblemStatus.svelte";
  import ResumeStatus from "../Panel/ResumeStatus.svelte";
  import ProgressFill from "../ProgressFill/ProgressFill.svelte";
  import TokenChips from "../TokenChips/TokenChips.svelte";

  const panel = $derived(copyState());
  const root = $derived(rootFor("copy") ?? "");
  const dest = $derived(S.settings.copyDest);
</script>

{#snippet behind()}
  {#if panel === "running"}
    <ProgressFill pct={S.copyP} />
    <span class="tagline">{S.runFanOut ? "Fan-out" : "Copy"}</span>
    <button class="corner" data-act="cancel" title="Stop">{@html ic.x()}</button>
  {/if}
{/snippet}

<Panel
  id="pCopy"
  dim={panel === "idle" && !S.resumeJournal && !S.src}
  status={panel === "running" || panel === "error" || panel === "done" ? panel : ""}
  {behind}
>
  {#if panel === "idle" && S.resumeJournal}
    <ResumeStatus name="copy" dest={0} />
  {:else if panel === "idle"}
    <h2>copy</h2>
    <div class="sub">Media files will be copied to:</div>
    <button class="btn sm" data-act="dest" data-w="copy">{@html ic.folder(14)} Choose…</button>
    {#if dest}<PathLine path={dest + (dest.endsWith(sep) ? "" : sep)} />{:else}<div class="path">No folder chosen</div>{/if}
    <TokenChips />
    <PreviewLine which="copy" />
  {:else if panel === "running"}
    <LiveStatus heading="copying to" {root} pct={S.copyP} cur={S.cur} rate={rateText()} />
  {:else if panel === "error"}
    <ProblemStatus heading="copy incomplete" {root} failed={S.failCopy} fallback="stopped" fallbackNote={S.abortReason ?? ""} logAct="log" />
  {:else}
    <DoneStatus heading="copied to" {root} bytes={totalBytes()} files={S.files.length} verb="copied" logAct="log" />
  {/if}
</Panel>
