<script lang="ts">
  import type { Settings } from "../../state.svelte.ts";

  let {
    on,
    setting,
    label,
    lock = false,
    ...attrs
  }: {
    on: boolean;
    /** The setting it flips, through the app's click handling. */
    setting?: keyof Settings;
    label: string;
    lock?: boolean;
    [attr: string]: unknown;
  } = $props();

  const act = $derived(setting && !lock);
</script>

<button class={["tog", on && "on", lock && "lock"]} data-act={act ? "tog" : undefined} data-k={act ? setting : undefined} aria-pressed={on} aria-label={label} {...attrs}></button>

<style src="./Toggle.css"></style>
