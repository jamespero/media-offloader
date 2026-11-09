<script lang="ts">
  import { ic, type IconName } from "../../icons";

  let {
    icon = null,
    label,
    act,
    disabled = false,
    mono = false,
    on = false,
    tall = false,
    marked,
    shortcut,
    hint,
    checked = false,
    ...attrs
  }: {
    icon?: IconName | null;
    label: string;
    act: string;
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
    [attr: string]: unknown;
  } = $props();
</script>

<button class={["mi", on && "on", tall && "tall"]} data-act={act} {disabled} {...attrs}>
  {#if marked !== undefined}
    <span class="mi-icon">{#if marked}{@html ic.check(14)}{/if}</span>
  {:else if icon}
    {@html ic[icon](15)}
  {:else}
    <span class="mi-icon"></span>
  {/if}
  <span>{#if mono}<span class="recent-path">{label}</span>{:else}{label}{/if}</span>
  {#if shortcut}<span class="kb">{shortcut}</span>{/if}
  {#if hint}<span class="sm">{hint}</span>{/if}
  {#if checked}<span class="ck">{@html ic.check(14)}</span>{/if}
</button>

<style src="./MenuItem.css"></style>
