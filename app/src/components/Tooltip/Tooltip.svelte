<script lang="ts">
  import { fmtSize } from "../../format";
  import { fileManager } from "../../state.svelte.ts";
  import { hovered } from "./tooltipState.svelte.ts";

  const OFFSET_X = 14;
  const OFFSET_Y = 16;
  const MARGIN = 8;

  const hover = $derived(hovered());
  const file = $derived(hover?.file ?? null);

  // Beside the pointer, and inside the window once its size for this file is known.
  let width = $state(0);
  let height = $state(0);
  let innerWidth = $state(0);
  let innerHeight = $state(0);
  const x = $derived(hover ? Math.min(hover.x + OFFSET_X, innerWidth - width - MARGIN) : 0);
  const y = $derived(hover ? Math.min(hover.y + OFFSET_Y, innerHeight - height - MARGIN) : 0);

  const status = $derived.by(() => {
    if (!file) return "";
    return {
      pending: "Not copied yet",
      copying: file.verify ? "Verifying…" : "Copying…",
      copied: "Copied & verified",
      backing: file.verify ? "Verifying backup…" : "Backing up…",
      backed: "Copied & backed up · verified",
      failed: file.failure ?? "Not verified",
    }[file.state];
  });
  const when = $derived(file?.modified ? new Date(file.modified) : null);
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<div
  class={["tip", file && "on"]}
  id="tip"
  style:--x="{x}px"
  style:--y="{y}px"
  bind:offsetWidth={width}
  bind:offsetHeight={height}
>
  {#if file}
    <b>{file.name}</b>{fmtSize(file.size)}{#if when}{" · "}{when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} {when.toTimeString().slice(0, 8)}{/if}
    <div class="h">{file.srcRel}</div>
    <div class={["state", file.state === "failed" ? "problem" : /backed|copied/.test(file.state) && "done"]}>{status}</div>
    <div class="h">Double-click to show in {fileManager}</div>
  {/if}
</div>

<style src="./Tooltip.css"></style>
