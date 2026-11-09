<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    note,
    mono = false,
    boxed = false,
    children,
  }: {
    title?: string;
    note: string;
    mono?: boolean;
    /** With a rule above it too, for use on its own inside a dialog. */
    boxed?: boolean;
    children?: Snippet;
  } = $props();
</script>

<div class={["set", boxed && "boxed"]}>
  <div class="l">
    {#if title}<b>{title}</b>{/if}
    <span>{#if mono}<span class="path">{note}</span>{:else}{note}{/if}</span>
  </div>
  {@render children?.()}
</div>

<style src="./SettingRow.css"></style>
