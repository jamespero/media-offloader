<script lang="ts">
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  let { onQuit }: { onQuit: () => void } = $props();
</script>

<Dialog>
  <h3>Stop and quit?</h3>
  <p>An offload is still running. Verified files are kept; the file in progress is removed.</p>
  <div class="acts">
    <Button onclick={closeOverlay}>Keep going</Button>
    <Button variant="danger" id="quitBtn" onclick={onQuit}>Stop and quit</Button>
  </div>
</Dialog>
