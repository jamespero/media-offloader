<script lang="ts">
  import { fmtSize, plural } from "../../format";
  import { day } from "../../history";
  import { ic } from "../../icons";
  import { S, totalBytes } from "../../state.svelte.ts";

  const past = $derived(S.view ? (S.detail?.item ?? null) : null);
</script>

<div class="foot">
  {#if past}
    <span><span class="title">{past.label}</span> · {day(past.started)}</span>
    <span class="count">{plural(past.files, "file")} · {fmtSize(past.bytes)}</span>
    <span class="items"><span>{@html ic.file(12)}History</span></span>
  {:else}
    <span>
      {#if S.src}<span class="title">{S.src.label}</span> · {S.src.media}{:else}No source{/if}
    </span>
    <span class="count">{S.src && S.plan ? `${plural(S.files.length, "file")} · ${fmtSize(totalBytes())}` : ""}</span>
    <span class="items">
      <span>{@html ic.shield(12)}xxHash64{S.settings.md5 ? " + MD5" : ""}</span>
      <span>{S.settings.mhl ? "MHL on" : "MHL off"}</span>
      <span>{S.settings.fanOut ? "Fan-out" : "Sequential"}</span>
    </span>
  {/if}
</div>

<style src="./Footer.css"></style>
