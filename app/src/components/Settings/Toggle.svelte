<script lang="ts">
  import type { Settings } from "../../state.svelte.ts";

  let { on, setting, label, lock = false }: { on: boolean; setting?: keyof Settings; label: string; lock?: boolean } = $props();
</script>

<button class={["tog", on && "on", lock && "lock"]} data-act={lock ? undefined : "tog"} data-k={lock ? undefined : setting} aria-pressed={on} aria-label={label}></button>
