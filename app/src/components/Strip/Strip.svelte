<script lang="ts">
  import Segmented from "../Segmented/Segmented.svelte";
  import IconButton from "../IconButton/IconButton.svelte";
  import { KINDS, gridKind, kindOf } from "../../fileKinds";
  import { clock, day, itemState } from "../../history";
  import { ic } from "../../icons";
  import { activePreset } from "../../presets";
  import { S, running, shownFiles } from "../../state.svelte.ts";

  const TABS = 8;

  /** Recent offloads, oldest on the left; Current comes after them. */
  const past = $derived(S.history.filter((h) => h.path !== S.currentJournal).slice(0, TABS).reverse());
  const dot = $derived(running() ? "busy" : S.phase === "done" ? "ok" : S.phase === "error" ? "err" : "");
  const when = $derived(S.finishedAt ?? S.startedAt ?? new Date());

  const files = $derived(shownFiles());
  const kinds = $derived(KINDS.map(([k, label]) => [k, label, files.filter((f) => kindOf(f.ext) === k).length] as const).filter(([, , n]) => n > 0));
  const shownKind = $derived(gridKind(files));
  const kindItems = $derived(
    [...kinds, ["all", "All", files.length] as const].map(([k, label, count]) => ({ label, count, on: shownKind === k, "data-act": "kind", "data-k": k })),
  );

  let tabs: HTMLElement;

  // Keep the Current tab in view when there's a long history.
  $effect(() => {
    void [past, S.view, S.phase];
    tabs.scrollLeft = tabs.scrollWidth;
  });
</script>

<div class="strip">
  <div class="tabs" id="tabs" bind:this={tabs}>
    {#each past as h (h.path)}
      <button class={["tab", S.view === h.path && "active"]} data-act="tab" data-path={h.path} title={h.label}>
        <span class={["dot", itemState(h)]}></span><span class="t">{clock(h.started)}</span><span class="d">{day(h.started)}</span>
        <span class="x" data-act="forget" data-path={h.path} title="Remove from history">{@html ic.x(11)}</span>
      </button>
    {/each}
    <button class={["tab", !S.view && "active"]} data-act="tab" data-path="">
      <span class={["dot", dot]}></span><span class="t">{S.finishedAt ? when.toTimeString().slice(0, 8) : "Current"}</span><span class="d">{when.toLocaleDateString(undefined, { month: "short", day: "numeric" })}</span>
    </button>
  </div>
  {#if kinds.length >= 2}
    <div class="kinds" id="kinds">
      <Segmented compact items={kindItems} />
    </div>
  {/if}
  {#if !S.view}
    <button class="preset" id="presetBtn" data-act="presets" title="Preset: subfolder pattern">
      {@html ic.star(13)}<span>Preset</span><b>{activePreset()?.name ?? "Custom"}</b>{@html ic.chev()}
    </button>
  {/if}
  <IconButton id="menuBtn" data-act="mainmenu" title="Menu" aria-label="Menu">{@html ic.menu()}</IconButton>
</div>

<style src="./Strip.css"></style>
