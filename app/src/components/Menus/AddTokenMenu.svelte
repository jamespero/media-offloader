<script lang="ts">
  import { TOKEN_TYPES } from "../../tokens";
  import MenuItem from "../Menu/MenuItem.svelte";
</script>

{#each TOKEN_TYPES as t}
  {#if "sep" in t}
    <div class="sep"></div>
  {:else}
    <MenuItem icon={t.icon} label={t.label} act="addtok" data-t={t.t}>
      {#if t.isNew}<span class="sm">new</span>{/if}
    </MenuItem>
  {/if}
{/each}
