<script lang="ts">
  import { onMount } from "svelte";
  import { activePreset } from "../../presets";
  import { S } from "../../state.svelte.ts";

  const suggestion = activePreset()?.name ?? S.src?.label ?? "";

  let input: HTMLInputElement;
  let save: HTMLButtonElement;

  onMount(() => {
    input.focus();
    input.select();
  });
</script>

<div class="dialog">
  <h3>Save preset</h3>
  <p>Saves the subfolder pattern, so the next card from this camera is one click. The destinations aren't saved: they stay as they are, set once per project.</p>
  <div class="field">
    <input
      class="tin"
      id="presetName"
      placeholder="e.g. Nikon ZR"
      value={suggestion}
      spellcheck="false"
      bind:this={input}
      onkeydown={(e) => {
        if (e.key === "Enter") save.click();
      }}
    />
  </div>
  <div class="acts">
    <button class="btn" data-act="closeov">Cancel</button>
    <button class="btn primary compact" data-act="dosavepreset" bind:this={save}>Save</button>
  </div>
</div>
