<script lang="ts">
  import Dialog from "./Dialog.svelte";
  let { version }: { version: string } = $props();
</script>

<Dialog>
  <h3>Media Offloader</h3>
  <p>Version {version} · open source under MIT or Apache-2.0</p>
  <p>Copies camera cards with checksum verification (xxHash64, optional MD5), re-reading every copy from disk, and never overwrites anything.</p>
  <p class="note">Fonts: Outfit, Inter and JetBrains Mono, under the SIL Open Font License.</p>
  <div class="acts">
    <button class="btn" data-act="openrepo">View on GitHub</button>
    <button class="btn" data-act="closeov">Close</button>
  </div>
</Dialog>
