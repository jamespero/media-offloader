<script lang="ts">
  import Dialog from "./Dialog.svelte";
</script>

<Dialog>
  <h3>Re-verify a folder</h3>
  <p>Pick a folder that has an <span class="mono">ascmhl</span> manifest (or a .md5 list). Every file is re-read from disk and compared against the checksums recorded when it was offloaded.</p>
  <p class="note">Use it to check an archive drive, or to find out which copy is good when two disagree. Nothing in the folder is changed.</p>
  <div class="acts">
    <button class="btn" data-act="closeov">Cancel</button>
    <button class="btn primary compact" data-act="rvchoose">Choose Folder…</button>
  </div>
</Dialog>
