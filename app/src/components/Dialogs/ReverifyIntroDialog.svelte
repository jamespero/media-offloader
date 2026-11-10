<script lang="ts">
  import Mono from "../Mono/Mono.svelte";
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { chooseAndReverify } from "../../reverify.svelte.ts";
</script>

<Dialog title="Re-verify a folder">
  <p>Pick a folder that has an <Mono>ascmhl</Mono> manifest (or a .md5 list). Every file is re-read from disk and compared against the checksums recorded when it was offloaded.</p>
  <p class="note">Use it to check an archive drive, or to find out which copy is good when two disagree. Nothing in the folder is changed.</p>
  {#snippet actions()}
    <Button onclick={closeOverlay}>Cancel</Button>
    <Button variant="primary" size="compact" onclick={() => {
        closeOverlay();
        chooseAndReverify();
      }}>Choose Folder…</Button>
  {/snippet}
</Dialog>

<style src="./ReverifyIntroDialog.css"></style>
