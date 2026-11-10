<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  let {
    narrow = false,
    element = $bindable(),
    value = $bindable(),
    ...attrs
  }: Omit<HTMLInputAttributes, "class" | "value"> & {
    /** For a short number rather than a line of text. */
    narrow?: boolean;
    element?: HTMLInputElement;
    value?: string | number;
  } = $props();
</script>

<input class={["tin", narrow && "narrow"]} bind:this={element} bind:value {...attrs} />

<style src="./TextInput.css"></style>
