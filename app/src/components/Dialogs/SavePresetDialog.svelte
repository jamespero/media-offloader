<script lang="ts">
  import TextInput from "../TextInput/TextInput.svelte";
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { onMount } from "svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { toast } from "../Toast/toastState.svelte.ts";
  import { activePreset, saveCurrentAs } from "../../presets";
  import { S } from "../../state.svelte.ts";

  let name = $state(activePreset()?.name ?? S.src?.label ?? "");

  let input = $state<HTMLInputElement>();

  function save() {
    if (saveCurrentAs(name)) toast(true, "Preset saved", name.trim());
  }

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
      bind:value={name}
      spellcheck="false"
      bind:element={input}
      onkeydown={(e: KeyboardEvent) => {
        if (e.key === "Enter") save();
      }}
    />
  </div>
  <div class="acts">
    <Button onclick={closeOverlay}>Cancel</Button>
    <Button variant="primary" size="compact" onclick={save}>Save</Button>
  </div>
</Dialog>
