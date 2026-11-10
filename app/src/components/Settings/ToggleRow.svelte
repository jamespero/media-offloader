<script lang="ts">
  import type { BoolSetting } from "../../settingEdits";
  import { S } from "../../state.svelte.ts";
  import SettingRow from "./SettingRow.svelte";
  import Toggle from "./Toggle.svelte";

  let { title, note, setting }: { title: string; note: string; setting: BoolSetting } = $props();
</script>

<SettingRow {title} {note}>
  <Toggle on={S.settings[setting]} {setting} label={title} />
</SettingRow>
