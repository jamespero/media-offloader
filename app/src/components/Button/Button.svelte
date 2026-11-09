<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    variant = "default",
    size = "md",
    start = false,
    element = $bindable(),
    children,
    ...attrs
  }: {
    /** `onStatus` sits on a panel colored by status, such as a problem panel. */
    variant?: "default" | "primary" | "danger" | "onStatus";
    /** `compact` keeps a primary button to the regular height. */
    size?: "md" | "sm" | "compact";
    /** A panel's main action, with a little more room above it. */
    start?: boolean;
    element?: HTMLButtonElement;
    children: Snippet;
    [attr: string]: unknown;
  } = $props();
</script>

<button
  class={["btn", variant === "primary" && "primary", variant === "danger" && "danger", variant === "onStatus" && "on-status", size === "sm" && "sm", size === "compact" && "compact", start && "start"]}
  bind:this={element}
  {...attrs}
>
  {@render children()}
</button>

<style src="./Button.css"></style>
