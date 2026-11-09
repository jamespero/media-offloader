<script lang="ts">
  import Button from "../Button/Button.svelte";
  import Dialog from "./Dialog.svelte";
  import { S } from "../../state.svelte.ts";

  const done = $derived(S.files.filter((f) => f.state === "copied" || f.state === "backed").length);
</script>

<Dialog>
  <h3>Stop this offload?</h3>
  <p>{done} of {S.files.length} files are already copied and verified. Those stay where they are.</p>
  <p>The file in progress will be removed so nothing half-written is left behind.</p>
  <div class="acts">
    <Button data-act="closeov">Keep going</Button>
    <Button variant="danger" data-act="dostop">Stop</Button>
  </div>
</Dialog>
