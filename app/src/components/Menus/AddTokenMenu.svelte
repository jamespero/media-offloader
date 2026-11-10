<script lang="ts">
  import { addToken } from "../../tokenEdits";
  import { TOKEN_TYPES } from "../../tokens";
  import MenuItem from "../Menu/MenuItem.svelte";
</script>

{#each TOKEN_TYPES as t}
  {#if "sep" in t}
    <div class="sep"></div>
  {:else}
    <MenuItem icon={t.icon} label={t.label} onclick={() => addToken(t.t)} hint={t.isNew ? "new" : undefined} />
  {/if}
{/each}
