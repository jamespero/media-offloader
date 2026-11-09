<script lang="ts">
  import Dialog from "./Dialog.svelte";
  let { onQuit }: { onQuit: () => void } = $props();
</script>

<Dialog>
  <h3>Stop and quit?</h3>
  <p>An offload is still running. Verified files are kept; the file in progress is removed.</p>
  <div class="acts">
    <button class="btn" data-act="closeov">Keep going</button>
    <button class="btn danger" id="quitBtn" onclick={onQuit}>Stop and quit</button>
  </div>
</Dialog>
