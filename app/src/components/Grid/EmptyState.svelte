<script lang="ts">
  import { ic } from "../../icons";

  let {
    title,
    note = "",
    art = false,
    inline = false,
  }: {
    title: string;
    note?: string;
    /** Shows the card-insert drawing above the title. */
    art?: boolean;
    /** Sits in the page's flow, under any banners, rather than filling the grid. */
    inline?: boolean;
  } = $props();
</script>

<div class={["empty", inline && "inline"]}>
  <div>
    {#if art}<div class="empty-art">{@html ic.cardInsert()}</div>{/if}
    <h3>{title}</h3>
    <div>{note}</div>
  </div>
</div>

<style src="./EmptyState.css"></style>
