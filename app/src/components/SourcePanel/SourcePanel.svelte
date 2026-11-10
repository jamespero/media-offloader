<script lang="ts">
  import Heading from "../Panel/Heading.svelte";
  import Note from "../Panel/Note.svelte";
  import SourceHead from "../Panel/SourceHead.svelte";
  import Icon from "../Icon/Icon.svelte";
  import Row from "../Row/Row.svelte";
  import StatusPill from "../StatusPill/StatusPill.svelte";
  import IconButton from "../IconButton/IconButton.svelte";
  import Button from "../Button/Button.svelte";
  import { plural } from "../../format";
  import { resumeLabel } from "../../history";
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
  <SourceHead
    icon={src.kind}
    label={src.label}
    meta={src.media}
    path={src.path}
    chevron={!running()}
    onclick={running() ? undefined : (e) => srcMenu(e.currentTarget)}
  >
    {#snippet extra()}
      {#if canEject}<IconButton small title="Eject" onclick={ejectSource}><Icon name="eject" size={14} /></IconButton>{/if}
    {/snippet}
  </SourceHead>
{/snippet}

<!-- After the copy verified: an Eject button, or confirmation that it was ejected. -->
{#snippet ejectRow(rest: string)}
  {#if S.ejected}
    <Note>Ejected — safe to remove. {rest}</Note>
  {:else}
    <Note>Safe to remove. {rest}</Note>
    {#if canEject}<Button size="sm" onclick={ejectSource}><Icon name="eject" size={13} /> Eject</Button>{/if}
  {/if}
{/snippet}

<Panel id="pSrc" source>
  {#if !S.src}
    <div class="waiting-icon">
      {#if S.scanning}<span class="spin"><Icon name="spinner" size={36} /></span>{:else}<Icon name="cardWaiting" />{/if}
    </div>
    <Heading text="source" small />
    <Note>{S.scanning ? "Reading the source…" : "Waiting for media"}</Note>
    <Button size="sm" onclick={(e: MouseEvent) => srcMenu(e.currentTarget as Element)}>Select a source <Icon name="chev" /></Button>
  {:else if S.phase === "ready" || S.phase === "waiting"}
    {@render head()}
    <Note>
      {#if S.scanning}Reading the source…{:else if res}Picks up where it stopped. Files already verified aren't copied again.{:else}Media files have not been copied from this source.{/if}
    </Note>
    <Button variant="primary" start onclick={startOffload} disabled={!can} title={why}>{startLabel}</Button>
  {:else if cardBusy}
    {@render head()}
    <Row above="sm"><span class="pulse"></span><Note strong>Copying from this source</Note></Row>
    <Note>Keep it connected until the copy is verified.</Note>
  {:else if S.phase === "backing"}
    {@render head()}
    <StatusPill tone="done"><Icon name="check" size={13} /> Copy verified</StatusPill>
    {@render ejectRow("The backup continues from the copy.")}
  {:else if S.phase === "done"}
    {@render head()}
    <StatusPill tone="done"><Icon name="check" size={13} /> Copied &amp; verified</StatusPill>
    {#if S.src.kind === "folder"}
      <Note>Media files have been copied and verified.</Note>
    {:else}
      {@render ejectRow("Media files have been copied and verified.")}
    {/if}
  {:else if S.released}
    {@render head()}
    <StatusPill tone="done"><Icon name="check" size={13} /> Copy verified</StatusPill>
    {@render ejectRow("The backup didn't finish.")}
  {:else}
    {@render head()}
    <StatusPill tone="problem"><Icon name="alert" size={13} /> {S.failCopy > 0 ? `${plural(S.failCopy, "file")} failed` : "Stopped"}</StatusPill>
    <Note>Keep this card. {S.failCopy > 0 ? "Some files didn't verify." : "The copy isn't complete."}</Note>
  {/if}
</Panel>

<style src="./SourcePanel.css"></style>
