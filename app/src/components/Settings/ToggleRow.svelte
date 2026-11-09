<script lang="ts">
  import { S, type Settings } from "../../state.svelte.ts";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";

  type BoolSetting = { [K in keyof Settings]: Settings[K] extends boolean ? K : never }[keyof Settings];

  let { title, note, setting }: { title: string; note: string; setting: BoolSetting } = $props();
</script>

<SettingRow {title} {note}>
  <Toggle on={S.settings[setting]} {setting} label={title} />
</SettingRow>
