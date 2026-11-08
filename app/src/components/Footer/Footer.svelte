<script lang="ts">
  import { ic } from "../../icons";
  import type { FooterProps } from "./footerState.svelte.ts";

  let { title, detail, count, items }: FooterProps = $props();
</script>

<div class="foot">
  <span>
    {#if title}<span class="title">{title}</span> · {detail}{:else}No source{/if}
  </span>
  <span class="count">{count}</span>
  <span class="items">
    {#each items as item (item.text)}
      <span>{#if item.icon}{@html ic[item.icon](12)}{/if}{item.text}</span>
    {/each}
  </span>
</div>

<style src="./Footer.css"></style>
