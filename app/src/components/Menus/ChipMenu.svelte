<script lang="ts">
  import TextInput from "../TextInput/TextInput.svelte";
  import { onMount, untrack } from "svelte";
  import { closeMenus } from "../Menu/menuState.svelte.ts";
  import { S, tokenContext } from "../../state.svelte.ts";
  import { moveToken, removeToken, setReel, setTokenFormat } from "../../tokenEdits";
  import { DATE_FMTS, DAY_FMTS, fmtDate, fmtDay, tokenInfo, tokenText } from "../../tokens";
  import MenuItem from "../Menu/MenuItem.svelte";

  let { index, onEdit }: { index: number; onEdit: () => void } = $props();

  const token = untrack(() => S.settings.tokens[index]);
  const ctx = tokenContext();
  const count = $derived(S.settings.tokens.length);
  const reel = $derived(token.v || "A");
  const dateSample = token.t === "now" ? ctx.now : (ctx.shotDate ?? ctx.now);
  const firstNumber = untrack(() => (token.t === "day" ? S.settings.shootDay : ctx.reelNo(token.v || "A")));
  const firstText = untrack(() => token.v ?? "");

  let textInput = $state<HTMLInputElement>();

  onMount(() => {
    textInput?.focus();
    textInput?.select();
  });

  function closeOnEnter(e: KeyboardEvent) {
    if (e.key === "Enter") closeMenus();
  }

  function setNumber(e: Event) {
    const n = Math.trunc(Number((e.currentTarget as HTMLInputElement).value));
    if (!Number.isFinite(n) || n < 1 || n > 999) return;
    if (token.t === "day") S.settings.shootDay = n;
    else S.settings.reels[token.v || "A"] = n;
    onEdit();
  }

  function setText(e: Event) {
    token.v = (e.currentTarget as HTMLInputElement).value;
    onEdit();
  }
</script>

<!-- A labelled number field: shoot day, next roll. -->
{#snippet numberRow(label: string, note: string)}
  <div class="num-row">
    <span class="label">{label}</span>
    <TextInput narrow id="numIn" type="number" min="1" max="999" value={firstNumber} oninput={setNumber} onkeydown={closeOnEnter} />
  </div>
  <div class="mnote">{note}</div>
{/snippet}

{#snippet format(f: string, sample: string, chosen: boolean)}
  <MenuItem marked={chosen} label={f} onclick={() => setTokenFormat(index, f)} hint={sample} />
{/snippet}

{#if token.t === "now" || token.t === "shot"}
  <div class="mh">{token.t === "now" ? "Now: current date/time" : "Shot: capture date/time"}</div>
  {#each DATE_FMTS as f}{@render format(f, fmtDate(dateSample, f), (token.fmt ?? "YY-MM-DD") === f)}{/each}
{:else if token.t === "day"}
  <div class="mh">Shoot day</div>
  {@render numberRow("Day", "Set by hand; it doesn't count up on its own.")}
  <div class="sep"></div>
  <div class="mh">Format</div>
  {#each DAY_FMTS as f}{@render format(f, fmtDay(S.settings.shootDay, f, ctx.shotDate ?? ctx.now), (token.fmt ?? "Day01") === f)}{/each}
{:else if token.t === "text" || token.t === "project"}
  <div class="mh">{token.t === "text" ? "Custom text" : "Project name"}</div>
  <div class="mi-field">
    <TextInput id="txtIn" value={firstText} spellcheck="false" bind:element={textInput} oninput={setText} onkeydown={closeOnEnter} />
  </div>
{:else if token.t === "reel"}
  <div class="mh">Reel letter</div>
  {#each ["A", "B", "C", "D"] as l}
    <MenuItem icon="tag" label="{l}{String(ctx.reelNo(l)).padStart(3, '0')}, {l}{String(ctx.reelNo(l) + 1).padStart(3, '0')}…" onclick={() => setReel(index, l)} checked={reel === l} />
  {/each}
  <div class="sep"></div>
  {@render numberRow(`Next ${reel} roll`, "Counts up after each completed offload. Change it to match the card's label.")}
{:else}
  <div class="mh">{tokenInfo(token.t).label}</div>
  <div class="mnote">Resolves to <span class="mono-value">{tokenText(token, ctx)}</span></div>
{/if}
<div class="sep"></div>
{#if index > 0}<MenuItem icon="chevL" label="Move Left" onclick={() => moveToken(index, -1)} />{/if}
{#if index < count - 1}<MenuItem icon="chevR" label="Move Right" onclick={() => moveToken(index, 1)} />{/if}
<MenuItem icon="trash" label="Remove" onclick={() => removeToken(index)} />

<style src="./ChipMenu.css"></style>
