<script lang="ts">
  import { plural } from "../../format";
  import { clock, day } from "../../history";
  import { ic } from "../../icons";
  import { resuming } from "../../panelState";
  import { S, running } from "../../state.svelte.ts";
  import Banner from "./Banner.svelte";
  import Grid from "./Grid.svelte";

  const res = $derived(resuming());
  const conflict = $derived(S.dests?.[0]);
  const problems = $derived(S.plan?.problems ?? []);
  const renamed = $derived(S.plan?.renamed ?? []);
  /** The last offload, when it didn't finish and nothing else has started. */
  const interrupted = $derived.by(() => {
    const last = S.history[0];
    return !last || last.outcome || running() || S.src || last.path === S.currentJournal ? null : last;
  });
</script>

{#if !S.src}
  {#if interrupted}
    {@const last = interrupted}
    <Banner icon={ic.alert()} tone="bad">
      <b>The offload of {last.label} from {day(last.started)} at {clock(last.started)} didn't finish.</b>
      {last.sourcePresent ? "Resume picks up where it stopped." : `Connect ${last.label} to resume it.`}
      {#snippet action()}
        {#if last.sourcePresent}
          <button class="btn sm" data-act="resume" data-path={last.path}>Resume</button>
        {:else}
          <button class="btn sm" data-act="tab" data-path={last.path}>View</button>
        {/if}
      {/snippet}
    </Banner>
  {/if}
  <div class="empty">
    <div>
      <div class="empty-art">{@html ic.cardInsert()}</div>
      <h3>{S.scanning ? "Reading…" : "Waiting for media"}</h3>
      <div>Insert a card or drive, or choose a folder to begin.</div>
    </div>
  </div>
{:else}
  {#if S.phase === "ready" && res}
    {@const when = new Date(res.started)}
    <Banner icon={ic.refresh()} tone="info">
      <b>Continuing the offload of {res.label}</b> from {when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} at {when.toTimeString().slice(0, 8)}. Files already verified (shown in mint) aren't copied again; nothing in those folders is overwritten.
    </Banner>
  {/if}
  {#if S.phase === "ready" && conflict?.conflict != null}
    <Banner icon={ic.shield()}>
      <b>{S.src.label} was already offloaded</b> to <span class="mono">{conflict.requested}</span>. Offloading again copies into a new <span class="mono">{conflict.root.split(/[\\/]/).pop()}</span> folder — nothing is overwritten.
    </Banner>
  {/if}
  {#if S.phase === "ready" && S.stoppedKept > 0}
    <Banner icon={ic.info()} tone="info">
      <b>Stopped.</b> {S.stoppedKept} verified file{S.stoppedKept === 1 ? " was" : "s were"} kept. The partial file was removed.
    </Banner>
  {/if}
  {#if S.phase === "error" && S.abortReason}
    <Banner icon={ic.alert()} tone="bad"><b>Stopped: {S.abortReason}.</b> Verified files were kept.</Banner>
  {/if}
  {#if problems.length}
    <Banner icon={ic.alert()} tone="bad">
      <b>{plural(problems.length, "item")} on this source can't be read</b> ({problems[0].rel}{problems.length > 1 ? ", …" : ""}). The offload can't be fully verified while they're unreadable.
    </Banner>
  {/if}
  {#if renamed.length && S.phase === "ready"}
    <Banner icon={ic.info()} tone="info">
      <b>{plural(renamed.length, "name")} will change</b> to fit the destination (e.g. <span class="mono">{renamed[0].from}</span> → <span class="mono">{renamed[0].to}</span>). The log lists every one.
    </Banner>
  {/if}
  {#if !S.files.length}
    <div class="empty inline">
      <div>
        <h3>{S.scanning ? "Reading…" : "No files"}</h3>
        <div>{S.scanning ? "" : "This source has no files to copy."}</div>
      </div>
    </div>
  {:else}
    <Grid files={S.files} />
  {/if}
{/if}
