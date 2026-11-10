<script lang="ts">
  import { ICONS, type IconName } from "../../icons";

  /** `size` and `stroke` default to the icon's own. */
  let { name, size, stroke }: { name: IconName; size?: number; stroke?: number } = $props();

  const icon = $derived(ICONS[name]);
  const px = $derived(size ?? icon.size);
</script>

<svg
  width={px}
  height={px}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke ?? icon.stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
>
  {#each icon.shapes as [tag, attrs]}
    {#if tag === "path"}<path {...attrs} />{:else if tag === "rect"}<rect {...attrs} />{:else}<circle {...attrs} />{/if}
  {/each}
</svg>

<style src="./Icon.css"></style>
