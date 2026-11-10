<script lang="ts">
  let {
    on = $bindable(false),
    label,
    lock = false,
    disabled = false,
    onchange,
  }: {
    /** Bind it to the setting it shows: `bind:on={S.settings.sound}`. */
    on?: boolean;
    label: string;
    /** Shown but never changeable, like a setting that's always on. */
    lock?: boolean;
    /** Not changeable for now, e.g. while an update installs. */
    disabled?: boolean;
    /** After it flips, for a setting with a side effect. */
    onchange?: (on: boolean) => void;
  } = $props();
</script>

<button
  class={["tog", on && "on", lock && "lock"]}
  onclick={() => {
    if (lock) return;
    on = !on;
    onchange?.(on);
  }}
  {disabled}
  aria-pressed={on}
  aria-label={label}
></button>

<style src="./Toggle.css"></style>
