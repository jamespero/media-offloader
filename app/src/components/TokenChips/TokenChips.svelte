<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import { tokensChanged } from "../../destinations";
  import { addTokenMenu, chipMenu } from "../../menus";
  import { S } from "../../state.svelte.ts";
  import { hasFormat, tokenInfo, type Token } from "../../tokens";

  /** The backup panel: says it follows the copy's subfolders, with nothing to edit. */
  let { sameAsCopy = false }: { sameAsCopy?: boolean } = $props();

  /** The small grey prefix on a chip, and what follows it. */
  function parts(t: Token): [string, string] {
    switch (t.t) {
      case "now":
        return ["Now", t.fmt ?? ""];
      case "shot":
        return ["Shot", t.fmt ?? ""];
      case "text":
        return ["", t.v || "text"];
      case "day":
        return ["Day", t.fmt ?? "Day01"];
      case "reel":
        return ["Reel", `${t.v || "A"}###`];
      case "project":
        return ["Project", t.v || "Project"];
      default:
        return ["", tokenInfo(t.t).label];
    }
  }
</script>

{#if sameAsCopy}
  <div class="lbl">Subfolders</div>
  <div class="chips"><span class="chip static"><Icon name="layers" size={13} /> Same as copy</span></div>
{:else}
  <div class="lbl">Add subfolders</div>
  <div class="chips">
    {#each S.settings.tokens as t, i}
      {@const [prefix, value] = parts(t)}
      <button class={["chip", t.t === "text" && "text"]} onclick={(e) => chipMenu(e.currentTarget, i, tokensChanged)}>
        {#if t.t !== "text"}<Icon name={tokenInfo(t.t).icon} />{/if}
        {#if prefix}<span class="k">{prefix}</span>{/if}
        {value}
        {#if hasFormat(t.t) || t.t === "reel"}<Icon name="chev" size={10} />{/if}
      </button>
    {/each}
    <button class="addchip" onclick={(e) => addTokenMenu(e.currentTarget)} title="Add subfolder"><Icon name="plus" /></button>
  </div>
{/if}

<style src="./TokenChips.css"></style>
