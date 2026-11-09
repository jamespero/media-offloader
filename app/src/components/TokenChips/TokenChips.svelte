<script lang="ts">
  import { ic } from "../../icons";
  import { S } from "../../state.svelte.ts";
  import { hasFormat, tokenInfo, type Token } from "../../tokens";

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

<div class="lbl">Add subfolders</div>
<div class="chips">
  {#each S.settings.tokens as t, i}
    {@const [prefix, value] = parts(t)}
    <button class={["chip", t.t === "text" && "text"]} data-act="chip" data-i={i}>
      {#if t.t !== "text"}{@html ic[tokenInfo(t.t).icon]()}{/if}
      {#if prefix}<span class="k">{prefix}</span>{/if}
      {value}
      {#if hasFormat(t.t) || t.t === "reel"}{@html ic.chev(10)}{/if}
    </button>
  {/each}
  <button class="addchip" data-act="addtoken" title="Add subfolder">{@html ic.plus()}</button>
</div>
