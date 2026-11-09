<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    id,
    status = "",
    dim = false,
    source = false,
    behind,
    children,
  }: {
    id: string;
    /** Colors the whole panel: working, done or problem. */
    status?: "" | "running" | "done" | "error";
    dim?: boolean;
    source?: boolean;
    /** Drawn around the content: the progress fill, corner buttons. */
    behind?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section {id} class={["panel", source && "src", dim && "dim", status]}>
  {@render behind?.()}
  <div class="content">{@render children()}</div>
</section>

<style src="./Panel.css"></style>
