<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import { shownToast } from "./toastState.svelte.ts";

  const message = $derived(shownToast());
</script>

{#if message}
  {#key message}
    <div class="toast">
      <div class={["ic", !message.ok && "bad"]}>{#if message.ok}<Icon name="check" size={18} stroke={2.6} />{:else}<Icon name="alert" size={18} />{/if}</div>
      <div><b>{message.title}</b><span>{message.body}</span></div>
    </div>
  {/key}
{/if}

<style src="./Toast.css"></style>
