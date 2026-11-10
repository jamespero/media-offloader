<script lang="ts">
  import { toggleSetting, type BoolSetting } from "../../settingEdits";

  let {
    on,
    setting,
    label,
    lock = false,
    ...attrs
  }: {
    on: boolean;
    /** The setting it flips. */
    setting?: BoolSetting;
    label: string;
    lock?: boolean;
    [attr: string]: unknown;
  } = $props();

</script>

<button
  class={["tog", on && "on", lock && "lock"]}
  onclick={() => {
    if (setting && !lock) toggleSetting(setting);
  }}
  aria-pressed={on}
  aria-label={label}
  {...attrs}
></button>

<style src="./Toggle.css"></style>
