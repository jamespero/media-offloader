<script lang="ts">
  import { fmtSize } from "../../format";
  import { fileManager, shownFiles, type FileView } from "../../state.svelte.ts";

  const OFFSET_X = 14;
  const OFFSET_Y = 16;
  const MARGIN = 8;

  let file = $state.raw<FileView | null>(null);
  let mouse = $state.raw({ x: 0, y: 0 });
  let tip: HTMLElement;

  function onmousemove(e: MouseEvent) {
    const t = (e.target as Element).closest<HTMLElement>(".thumb");
    file = (t ? shownFiles()[+t.dataset.i!] : undefined) ?? null;
    mouse = { x: e.clientX, y: e.clientY };
  }

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

  // Keeps it beside the pointer and inside the window, once its size for this file is known.
  $effect(() => {
    if (!file) return;
    tip.style.setProperty("--x", Math.min(mouse.x + OFFSET_X, innerWidth - tip.offsetWidth - MARGIN) + "px");
    tip.style.setProperty("--y", Math.min(mouse.y + OFFSET_Y, innerHeight - tip.offsetHeight - MARGIN) + "px");
  });
</script>

<svelte:document {onmousemove} />

<div class={["tip", file && "on"]} id="tip" bind:this={tip}>
  {#if file}
    <b>{file.name}</b>{fmtSize(file.size)}{#if when}{" · "}{when.toLocaleDateString(undefined, { month: "short", day: "numeric" })} {when.toTimeString().slice(0, 8)}{/if}
    <div class="h">{file.srcRel}</div>
    <div class={["state", file.state === "failed" ? "problem" : /backed|copied/.test(file.state) && "done"]}>{status}</div>
    <div class="h">Double-click to show in {fileManager}</div>
  {/if}
</div>

<style src="./Tooltip.css"></style>
