<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import Row from "../Row/Row.svelte";
  import Button from "../Button/Button.svelte";
  import { deletePreset } from "../../presets";
  import { stepImportNo } from "../../tokenEdits";
  import { S, sep, tokenContext } from "../../state.svelte.ts";
  import { tokenText } from "../../tokens";
  import SettingRow from "./SettingRow.svelte";

  const ctx = $derived(tokenContext());

  function folders(tokens: (typeof S.settings.tokens)) {
    const path = tokens.map((t) => tokenText(t, ctx)).join(sep);
    return path ? `…${sep}${path}` : "no subfolders";
  }
</script>

<h5>naming &amp; presets</h5>
<p class="intro">A preset saves a subfolder pattern, e.g. one per camera. It goes under the copy and backup destinations, which you set once per project. Pick a preset from the ★ Preset button above the thumbnails.</p>
{#each S.settings.presets as p (p.id)}
  <SettingRow title={p.name} note={folders(p.tokens)} mono>
    <Button size="sm" onclick={() => deletePreset(p.id)} title="Delete preset"><Icon name="trash" size={13} /></Button>
  </SettingRow>
{:else}
  <SettingRow note="No presets yet. Use ★ Preset → Save Current as Preset…." />
{/each}
<SettingRow title="Import # counter" note="The next import number. Counts up after each completed offload.">
  <Row gap="tight">
    <Button size="sm" onclick={() => stepImportNo(-1)} aria-label="Lower">−</Button>
    <span class="stepper-value">{String(S.settings.importNo).padStart(3, "0")}</span>
    <Button size="sm" onclick={() => stepImportNo(1)} aria-label="Raise">+</Button>
  </Row>
</SettingRow>

<style src="./NamingPane.css"></style>
