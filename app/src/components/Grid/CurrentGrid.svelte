<script lang="ts">
  import Mono from "../Mono/Mono.svelte";
  import Button from "../Button/Button.svelte";
  import { plural } from "../../format";
  import { clock, day, showTab } from "../../history";
  import { resuming } from "../../panelState";
  import { resumeFrom } from "../../source";
  import { S, running } from "../../state.svelte.ts";
  import Banner from "./Banner.svelte";
  import EmptyState from "./EmptyState.svelte";
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
    <Banner icon="alert" tone="bad">
      {#snippet lead()}The offload of {last.label} from {day(last.started)} at {clock(last.started)} didn't finish.{/snippet}
      {last.sourcePresent ? "Resume picks up where it stopped." : `Connect ${last.label} to resume it.`}
      {#snippet action()}
        {#if last.sourcePresent}
          <Button size="sm" onclick={() => resumeFrom(last.path)}>Resume</Button>
        {:else}
          <Button size="sm" onclick={() => showTab(last.path)}>View</Button>
        {/if}
      {/snippet}
    </Banner>
  {/if}
  <EmptyState art title={S.scanning ? "Reading…" : "Waiting for media"} note="Insert a card or drive, or choose a folder to begin." />
{:else}
  {@const src = S.src}
  {#if S.phase === "ready" && res}
    {@const when = new Date(res.started)}
    <Banner icon="refresh" tone="info">
      {#snippet lead()}Continuing the offload of {res.label}{/snippet}
      from {when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} at {when.toTimeString().slice(0, 8)}. Files already verified (shown in mint) aren't copied again; nothing in those folders is overwritten.
    </Banner>
  {/if}
  {#if S.phase === "ready" && conflict?.conflict != null}
    <Banner icon="shield">
      {#snippet lead()}{src.label} was already offloaded{/snippet}
      to <Mono>{conflict.requested}</Mono>. Offloading again copies into a new <Mono>{conflict.root.split(/[\\/]/).pop()}</Mono> folder — nothing is overwritten.
    </Banner>
  {/if}
  {#if S.phase === "ready" && S.stoppedKept > 0}
    <Banner icon="info" tone="info">
      {#snippet lead()}Stopped.{/snippet}
      {S.stoppedKept} verified file{S.stoppedKept === 1 ? " was" : "s were"} kept. The partial file was removed.
    </Banner>
  {/if}
  {#if S.phase === "error" && S.abortReason}
    <Banner icon="alert" tone="bad">
      {#snippet lead()}Stopped: {S.abortReason}.{/snippet}
      Verified files were kept.
    </Banner>
  {/if}
  {#if problems.length}
    <Banner icon="alert" tone="bad">
      {#snippet lead()}{plural(problems.length, "item")} on this source can't be read{/snippet}
      ({problems[0].rel}{problems.length > 1 ? ", …" : ""}). The offload can't be fully verified while they're unreadable.
    </Banner>
  {/if}
  {#if renamed.length && S.phase === "ready"}
    <Banner icon="info" tone="info">
      {#snippet lead()}{plural(renamed.length, "name")} will change{/snippet}
      to fit the destination (e.g. <Mono>{renamed[0].from}</Mono> → <Mono>{renamed[0].to}</Mono>). The log lists every one.
    </Banner>
  {/if}
  {#if !S.files.length}
    <EmptyState inline title={S.scanning ? "Reading…" : "No files"} note={S.scanning ? "" : "This source has no files to copy."} />
  {:else}
    <Grid files={S.files} />
  {/if}
{/if}
