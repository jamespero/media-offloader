<script lang="ts">
  import { ic } from "../../icons";
  import { shownToast } from "./toastState.svelte.ts";

  const message = $derived(shownToast());
</script>

{#if message}
  {#key message}
    <div class="toast">
      <div class={["ic", !message.ok && "bad"]}>{@html message.ok ? ic.check(18, 2.6) : ic.alert(18)}</div>
      <div><b>{message.title}</b><span>{message.body}</span></div>
    </div>
  {/key}
{/if}
