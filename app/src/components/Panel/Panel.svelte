<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../Icon/Icon.svelte";
  import ProgressFill from "../ProgressFill/ProgressFill.svelte";

  /**
   * One of the three panels across the top. While something runs it can show `progress`
   * behind its content, a `tagline` in one corner and a `corner` button (an ×) in the other.
   */
  let {
    id,
    status = "",
    dim = false,
    source = false,
    progress,
    tagline,
    corner,
    children,
  }: {
    id: string;
    /** Colors the whole panel: working, done or problem. */
    status?: "" | "running" | "done" | "error";
    dim?: boolean;
    source?: boolean;
    progress?: number;
    tagline?: string;
    corner?: { title: string; onclick: () => void; iconSize?: number };
    children: Snippet;
  } = $props();
</script>

<section {id} class={["panel", source && "src", dim && "dim", status]}>
  {#if progress !== undefined}<ProgressFill pct={progress} />{/if}
  {#if tagline}<span class="tagline">{tagline}</span>{/if}
  {#if corner}<button class="corner" onclick={corner.onclick} title={corner.title}><Icon name="x" size={corner.iconSize} /></button>{/if}
  <div class="content">{@render children()}</div>
</section>

<style src="./Panel.css"></style>
