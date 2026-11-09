<script lang="ts">
  import type { Snippet } from "svelte";
  import { ic, type IconName } from "../../icons";

  let {
    icon = null,
    label,
    act,
    disabled = false,
    mono = false,
    children,
    ...attrs
  }: {
    icon?: IconName | null;
    label: string;
    act: string;
    disabled?: boolean;
    /** Shows the label as a path. */
    mono?: boolean;
    /** What sits at the right edge: a shortcut, a size, a check. */
    children?: Snippet;
    [attr: string]: unknown;
  } = $props();
</script>

<button class="mi" data-act={act} {disabled} {...attrs}>
  {#if icon}{@html ic[icon](15)}{:else}<span class="mi-icon"></span>{/if}
  <span>{#if mono}<span class="recent-path">{label}</span>{:else}{label}{/if}</span>
  {@render children?.()}
</button>
