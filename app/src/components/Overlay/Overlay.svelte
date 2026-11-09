<script lang="ts">
  import { closeOverlay, openedOverlay } from "./overlayState.svelte.ts";

  const open = $derived(openedOverlay());
  let scrim = $state<HTMLElement>();
</script>

{#if open}
  {#key open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="scrim"
      id="scrim"
      bind:this={scrim}
      onmousedown={(e) => {
        if (e.target === scrim) closeOverlay();
      }}
    >
      <open.component {...open.props} />
    </div>
  {/key}
{/if}

<style src="./Overlay.css"></style>
