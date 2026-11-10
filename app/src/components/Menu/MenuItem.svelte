<script lang="ts">
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "../Icon/Icon.svelte";
  import type { IconName } from "../../icons";
  import { closeMenus } from "./menuState.svelte.ts";

  let {
    icon = null,
    label,
    onclick,
    disabled = false,
    mono = false,
    on = false,
    tall = false,
    marked,
    shortcut,
    hint,
    checked = false,
    ...attrs
  }: Omit<HTMLButtonAttributes, "class" | "children" | "onclick"> & {
    icon?: IconName | null;
    label: string;
    /** Runs, then the menu it's in closes. (Closing first would take away the menu's props.) */
    onclick: () => void;
    disabled?: boolean;
    /** Shows the label as a path. */
    mono?: boolean;
    /** The current tab, in a list of them. */
    on?: boolean;
    tall?: boolean;
    /** Leads with a check when true, or a blank when false, instead of an icon. */
    marked?: boolean;
    /** A keyboard shortcut at the right edge. */
    shortcut?: string;
    /** A small note at the right edge, such as a size. */
    hint?: string;
    /** A check at the right edge. */
    checked?: boolean;
  } = $props();
</script>

<button
  class={["mi", on && "on", tall && "tall"]}
  {disabled}
  {...attrs}
  onclick={() => {
    onclick();
    closeMenus();
  }}
>
  {#if marked !== undefined}
    <span class="mi-icon">{#if marked}<Icon name="check" size={14} />{/if}</span>
  {:else if icon}
    <Icon name={icon} size={15} />
  {:else}
    <span class="mi-icon"></span>
  {/if}
  <span>{#if mono}<span class="recent-path">{label}</span>{:else}{label}{/if}</span>
  {#if shortcut}<span class="kb">{shortcut}</span>{/if}
  {#if hint}<span class="sm">{hint}</span>{/if}
  {#if checked}<span class="ck"><Icon name="check" size={14} /></span>{/if}
</button>

<style src="./MenuItem.css"></style>
