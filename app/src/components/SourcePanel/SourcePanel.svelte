<script lang="ts">
  import Row from "../Row/Row.svelte";
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import IconButton from "../IconButton/IconButton.svelte";
  import Button from "../Button/Button.svelte";
  import { plural } from "../../format";
  import { resumeLabel } from "../../history";
  import { ic } from "../../icons";
  import { resuming } from "../../panelState";
  import { srcMenu } from "../../menus";
  import { startOffload } from "../../offload";
  import { ejectEnabled, ejectSource } from "../../source";
  import { S, running } from "../../state.svelte.ts";
  import Panel from "../Panel/Panel.svelte";

  /** Whether the card is being read right now (so it mustn't be ejected). */
  const cardBusy = $derived(S.phase === "copying" || (S.phase === "backing" && S.runFanOut));
  const canEject = $derived(ejectEnabled && !!S.src && S.src.kind !== "folder" && !S.ejected && !cardBusy && !S.scanning);
  const res = $derived(resuming());
  const can = $derived((!!S.settings.copyDest || !!S.resumeJournal) && !S.scanning && !S.prepError);
  const why = $derived(S.prepError ? "Fix the destination first." : !S.settings.copyDest && !res ? "Choose where to copy first." : "");
  const startLabel = $derived(res ? resumeLabel(res) : S.dests?.[0]?.conflict != null ? "Offload again" : "Start");
</script>

{#snippet head()}
  {@const src = S.src!}
  <div class="srcicon">{@html src.kind === "card" ? ic.card() : src.kind === "drive" ? ic.drive() : ic.folder()}</div>
  <h2>source</h2>
  <Row gap="tighter">
    <button class="vol" onclick={running() ? undefined : (e) => srcMenu(e.currentTarget)} title={src.path}>
      <span>{src.label}</span>
      {#if !running()}{@html ic.chev()}{/if}
    </button>
    {#if canEject}<IconButton small title="Eject" onclick={ejectSource}>{@html ic.eject(14)}</IconButton>{/if}
  </Row>
  <div class="meta">{src.media}</div>
{/snippet}

<!-- After the copy verified: an Eject button, or confirmation that it was ejected. -->
{#snippet ejectRow(rest: string)}
  {#if S.ejected}
    <div class="sub">Ejected — safe to remove. {rest}</div>
  {:else}
    <div class="sub">Safe to remove. {rest}</div>
    {#if canEject}<Button size="sm" onclick={ejectSource}>{@html ic.eject(13)} Eject</Button>{/if}
  {/if}
{/snippet}

<Panel id="pSrc" source>
  {#if !S.src}
    <div class="waiting-icon">
      {#if S.scanning}<span class="spin">{@html ic.spinner(36)}</span>{:else}{@html ic.cardWaiting()}{/if}
    </div>
    <h2>source</h2>
    <div class="sub">{S.scanning ? "Reading the source…" : "Waiting for media"}</div>
    <Button size="sm" onclick={(e: MouseEvent) => srcMenu(e.currentTarget as Element)}>Select a source {@html ic.chev()}</Button>
  {:else if S.phase === "ready" || S.phase === "waiting"}
    {@render head()}
    <div class="sub">
      {#if S.scanning}Reading the source…{:else if res}Picks up where it stopped. Files already verified aren't copied again.{:else}Media files have not been copied from this source.{/if}
    </div>
    <Button variant="primary" start onclick={startOffload} disabled={!can} title={why}>{startLabel}</Button>
  {:else if cardBusy}
    {@render head()}
    <Row above="sm"><span class="pulse"></span><span class="sub strong">Copying from this source</span></Row>
    <div class="sub">Keep it connected until the copy is verified.</div>
  {:else if S.phase === "backing"}
    {@render head()}
    <StatusPill tone="done">{@html ic.check(13)} Copy verified</StatusPill>
    {@render ejectRow("The backup continues from the copy.")}
  {:else if S.phase === "done"}
    {@render head()}
    <StatusPill tone="done">{@html ic.check(13)} Copied &amp; verified</StatusPill>
    {#if S.src.kind === "folder"}
      <div class="sub">Media files have been copied and verified.</div>
    {:else}
      {@render ejectRow("Media files have been copied and verified.")}
    {/if}
  {:else if S.released}
    {@render head()}
    <StatusPill tone="done">{@html ic.check(13)} Copy verified</StatusPill>
    {@render ejectRow("The backup didn't finish.")}
  {:else}
    {@render head()}
    <StatusPill tone="problem">{@html ic.alert(13)} {S.failCopy > 0 ? `${plural(S.failCopy, "file")} failed` : "Stopped"}</StatusPill>
    <div class="sub">Keep this card. {S.failCopy > 0 ? "Some files didn't verify." : "The copy isn't complete."}</div>
  {/if}
</Panel>
