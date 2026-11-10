<script lang="ts">
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";

  let {
    title,
    note,
    on = $bindable(false),
    onchange,
  }: { title: string; note: string; on?: boolean; onchange?: (on: boolean) => void } = $props();
</script>

<SettingRow {title} {note}>
  <Toggle bind:on label={title} {onchange} />
</SettingRow>
