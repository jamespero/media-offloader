<script lang="ts">
  import { closeMenus, openedMenu, type OpenMenu } from "./menuState.svelte.ts";

  const GAP = 6;
  const MARGIN = 8;

  const open = $derived(openedMenu());
  let menu = $state<HTMLElement>();

  /** A click anywhere outside the menu closes it, before that click does anything else. */
  function onclickcapture(e: MouseEvent) {
    if (menu && !menu.contains(e.target as Node)) closeMenus();
  }

  /** Places the menu under its anchor, or above it when there's no room below. */
  function place(el: HTMLElement, m: OpenMenu) {
    const r = m.anchor;
    let x = m.align === "right" ? r.right - el.offsetWidth : r.left;
    let y = r.bottom + GAP;
    if (y + el.offsetHeight > innerHeight - MARGIN) y = Math.max(MARGIN, r.top - el.offsetHeight - GAP);
    x = Math.max(MARGIN, Math.min(x, innerWidth - el.offsetWidth - MARGIN));
    el.style.setProperty("--x", x + "px");
    el.style.setProperty("--y", y + "px");
  }
</script>

<svelte:window {onclickcapture} onresize={closeMenus} />

{#if open}
  {#key open}
    <div class="menu" use:place={open} bind:this={menu}>
      <open.component {...open.props} />
    </div>
  {/key}
{/if}

<style src="./Menu.css"></style>
