<script lang="ts">
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { openRepo } from "../../places";
  let { version }: { version: string } = $props();
</script>

<Dialog>
  <h3>Media Offloader</h3>
  <p>Version {version} · open source under MIT or Apache-2.0</p>
  <p>Copies camera cards with checksum verification (xxHash64, optional MD5), re-reading every copy from disk, and never overwrites anything.</p>
  <p class="note">Fonts: Outfit, Inter and JetBrains Mono, under the SIL Open Font License.</p>
  <div class="acts">
    <Button onclick={openRepo}>View on GitHub</Button>
    <Button onclick={closeOverlay}>Close</Button>
  </div>
</Dialog>
