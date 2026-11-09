<script lang="ts">
  import { kindOf } from "../../fileKinds";
  import { ic } from "../../icons";
  import type { FileView } from "../../state.svelte.ts";
  import { thumbUrl } from "../../thumbs";

  let { file, index }: { file: FileView; index: number } = $props();

  const CLASS = { pending: "pending", copying: "active", copied: "ok", backed: "ok", backing: "bk-active", failed: "fail" };
  const ART = { video: () => ic.film(30), stills: () => ic.image(30), audio: () => ic.wave(30), other: () => ic.file(28) };

  const url = $derived(thumbUrl(file));
  const check = ic.check(10, 3);
</script>

<div class={["thumb", CLASS[file.state]]} data-i={index} style:--p="{file.p}%">
  <div class="img">
    {#if url}<img src={url} alt="" />{:else}{@html ART[kindOf(file.ext)]()}{/if}
  </div>
  <div class="shade"></div>
  {#if file.ext}<span class="ty">{file.ext.slice(0, 5)}</span>{/if}
  <span class="st">
    {#if file.state === "copying"}{file.verify ? "verify" : "copy"}
    {:else if file.state === "copied"}{@html check}
    {:else if file.state === "backing"}{file.verify ? "verify" : "backup"}
    {:else if file.state === "backed"}{@html check}{@html check}
    {:else if file.state === "failed"}!{/if}
  </span>
  <div class="bar"></div>
  <div class="nm">{file.name}</div>
  {#if file.flash}
    <div class="flash"><div><span>{file.flash}</span><i>{@html ic.check(18, 3)}</i></div></div>
  {/if}
</div>
