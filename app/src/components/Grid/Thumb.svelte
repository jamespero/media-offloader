<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import { kindOf } from "../../fileKinds";
  import { revealFile } from "../../places";
  import type { FileView } from "../../state.svelte.ts";
  import { thumbUrl } from "../../thumbs";

  let { file, index }: { file: FileView; index: number } = $props();

  const CLASS = { pending: "pending", copying: "active", copied: "ok", backed: "ok", backing: "bk-active", failed: "fail" };
  /** The placeholder for each kind of file, until (or unless) there's a thumbnail. */
  const ART = { video: ["film", 30], stills: ["image", 30], audio: ["wave", 30], other: ["file", 28] } as const;

  const url = $derived(thumbUrl(file));
</script>

{#snippet check()}<Icon name="check" size={10} stroke={3} />{/snippet}

<!-- Double-click shows the file in Explorer or Finder; the tooltip says so. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class={["thumb", CLASS[file.state]]} data-i={index} style:--p="{file.p}%" ondblclick={() => revealFile(file)}>
  <div class="img">
    {#if url}<img src={url} alt="" />{:else}{@const [art, size] = ART[kindOf(file.ext)]}<Icon name={art} {size} />{/if}
  </div>
  <div class="shade"></div>
  {#if file.ext}<span class="ty">{file.ext.slice(0, 5)}</span>{/if}
  <span class="st">
    {#if file.state === "copying"}{file.verify ? "verify" : "copy"}
    {:else if file.state === "copied"}{@render check()}
    {:else if file.state === "backing"}{file.verify ? "verify" : "backup"}
    {:else if file.state === "backed"}{@render check()}{@render check()}
    {:else if file.state === "failed"}!{/if}
  </span>
  <div class="bar"></div>
  <div class="nm">{file.name}</div>
  {#if file.flash}
    <div class="flash"><div><span>{file.flash}</span><i><Icon name="check" size={18} stroke={3} /></i></div></div>
  {/if}
</div>

<style src="./Thumb.css"></style>
