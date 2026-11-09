<script lang="ts">
  import { plural } from "../../format";
  import { resumeLabel } from "../../history";
  import { ic } from "../../icons";
  import { resuming } from "../../panelState";
  import { S, running } from "../../state.svelte.ts";
  import Panel from "../Panel/Panel.svelte";

  /** Whether the card is being read right now (so it mustn't be ejected). */
  const cardBusy = $derived(S.phase === "copying" || (S.phase === "backing" && S.runFanOut));
  const canEject = $derived(!!S.src && S.src.kind !== "folder" && !S.ejected && !cardBusy && !S.scanning);
  const res = $derived(resuming());
  const can = $derived((!!S.settings.copyDest || !!S.resumeJournal) && !S.scanning && !S.prepError);
  const why = $derived(S.prepError ? "Fix the destination first." : !S.settings.copyDest && !res ? "Choose where to copy first." : "");
  const startLabel = $derived(res ? resumeLabel(res) : S.dests?.[0]?.conflict != null ? "Offload again" : "Start");
</script>

{#snippet head()}
  {@const src = S.src!}
  <div class="srcicon">{@html src.kind === "card" ? ic.card() : src.kind === "drive" ? ic.drive() : ic.folder()}</div>
  <h2>source</h2>
  <div class="row tighter">
    <button class="vol" data-act={running() ? undefined : "srcmenu"} title={src.path}>
      <span>{src.label}</span>
      {#if !running()}{@html ic.chev()}{/if}
    </button>
    {#if canEject}<button class="iconbtn sm" title="Eject" data-act="eject">{@html ic.eject(14)}</button>{/if}
  </div>
  <div class="meta">{src.media}</div>
{/snippet}

<!-- After the copy verified: an Eject button, or confirmation that it was ejected. -->
{#snippet ejectRow(rest: string)}
  {#if S.ejected}
    <div class="sub">Ejected — safe to remove. {rest}</div>
  {:else}
    <div class="sub">Safe to remove. {rest}</div>
    {#if canEject}<button class="btn sm" data-act="eject">{@html ic.eject(13)} Eject</button>{/if}
  {/if}
{/snippet}

<Panel id="pSrc" source>
  {#if !S.src}
    <div class="waiting-icon">
      {#if S.scanning}<span class="spin">{@html ic.spinner(36)}</span>{:else}{@html ic.cardWaiting()}{/if}
    </div>
    <h2>source</h2>
    <div class="sub">{S.scanning ? "Reading the source…" : "Waiting for media"}</div>
    <button class="btn sm" data-act="srcmenu">Select a source {@html ic.chev()}</button>
  {:else if S.phase === "ready" || S.phase === "waiting"}
    {@render head()}
    <div class="sub">
      {#if S.scanning}Reading the source…{:else if res}Picks up where it stopped. Files already verified aren't copied again.{:else}Media files have not been copied from this source.{/if}
    </div>
    <button class="btn primary start" data-act="start" disabled={!can} title={why}>{startLabel}</button>
  {:else if cardBusy}
    {@render head()}
    <div class="row in-progress"><span class="pulse"></span><span class="sub strong">Copying from this source</span></div>
    <div class="sub">Keep it connected until the copy is verified.</div>
  {:else if S.phase === "backing"}
    {@render head()}
    <span class="safe">{@html ic.check(13)} Copy verified</span>
    {@render ejectRow("The backup continues from the copy.")}
  {:else if S.phase === "done"}
    {@render head()}
    <span class="safe">{@html ic.check(13)} Copied &amp; verified</span>
    {#if S.src.kind === "folder"}
      <div class="sub">Media files have been copied and verified.</div>
    {:else}
      {@render ejectRow("Media files have been copied and verified.")}
    {/if}
  {:else if S.released}
    {@render head()}
    <span class="safe">{@html ic.check(13)} Copy verified</span>
    {@render ejectRow("The backup didn't finish.")}
  {:else}
    {@render head()}
    <span class="bad">{@html ic.alert(13)} {S.failCopy > 0 ? `${plural(S.failCopy, "file")} failed` : "Stopped"}</span>
    <div class="sub">Keep this card. {S.failCopy > 0 ? "Some files didn't verify." : "The copy isn't complete."}</div>
  {/if}
</Panel>
