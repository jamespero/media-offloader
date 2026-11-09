<script lang="ts">
  import TextInput from "../TextInput/TextInput.svelte";
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { onMount } from "svelte";
  import { activePreset } from "../../presets";
  import { S } from "../../state.svelte.ts";

  const suggestion = activePreset()?.name ?? S.src?.label ?? "";

  let input = $state<HTMLInputElement>();
  let save = $state<HTMLButtonElement>();

  onMount(() => {
    input?.focus();
    input?.select();
  });
</script>

<Dialog>
  <h3>Save preset</h3>
  <p>Saves the subfolder pattern, so the next card from this camera is one click. The destinations aren't saved: they stay as they are, set once per project.</p>
  <div class="field">
    <TextInput
      id="presetName"
      placeholder="e.g. Nikon ZR"
      value={suggestion}
      spellcheck="false"
      bind:element={input}
      onkeydown={(e: KeyboardEvent) => {
        if (e.key === "Enter") save?.click();
      }}
    />
  </div>
  <div class="acts">
    <Button data-act="closeov">Cancel</Button>
    <Button variant="primary" size="compact" data-act="dosavepreset" bind:element={save}>Save</Button>
  </div>
</Dialog>
