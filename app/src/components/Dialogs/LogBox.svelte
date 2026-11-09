<script lang="ts">
  import type { Snippet } from "svelte";

  /** `fit` sizes it to its content rather than filling the window. */
  let { fit = false, children }: { fit?: boolean; children: Snippet } = $props();
</script>

<div class={["logbox", fit && "fit"]}>{@render children()}</div>

<style src="./LogBox.css"></style>
