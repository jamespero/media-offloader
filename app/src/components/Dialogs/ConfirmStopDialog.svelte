<script lang="ts">
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { stopOffload } from "../../offload";
  import { S } from "../../state.svelte.ts";

  const done = $derived(S.files.filter((f) => f.state === "copied" || f.state === "backed").length);
</script>

<Dialog title="Stop this offload?">
  <p>{done} of {S.files.length} files are already copied and verified. Those stay where they are.</p>
  <p>The file in progress will be removed so nothing half-written is left behind.</p>
  {#snippet actions()}
    <Button onclick={closeOverlay}>Keep going</Button>
    <Button variant="danger" onclick={stopOffload}>Stop</Button>
  {/snippet}
</Dialog>
