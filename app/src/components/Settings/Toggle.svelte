<script lang="ts">
  import { toggleSetting, type BoolSetting } from "../../settingEdits";

  let {
    on,
    setting,
    label,
    lock = false,
    disabled = false,
  }: {
    on: boolean;
    /** The setting it flips. */
    setting?: BoolSetting;
    label: string;
    lock?: boolean;
    /** Shown but not changeable for now, e.g. while an update installs. */
    disabled?: boolean;
  } = $props();

</script>

<button
  class={["tog", on && "on", lock && "lock"]}
  onclick={() => {
    if (setting && !lock) toggleSetting(setting);
  }}
  {disabled}
  aria-pressed={on}
  aria-label={label}
></button>

<style src="./Toggle.css"></style>
