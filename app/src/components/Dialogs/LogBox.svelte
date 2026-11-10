<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../Icon/Icon.svelte";
  import IconButton from "../IconButton/IconButton.svelte";
  import type { IconName } from "../../icons";

  /**
   * A large sheet with a header (icon, title, `extra` such as a button, and Close) over its
   * content. `fit` sizes it to its content rather than filling the window.
   */
  let {
    icon,
    title,
    fit = false,
    onclose,
    extra,
    children,
  }: { icon: IconName; title: string; fit?: boolean; onclose: () => void; extra?: Snippet; children: Snippet } = $props();
</script>

<div class={["logbox", fit && "fit"]}>
  <header>
    <Icon name={icon} /><b>{title}</b>
    {@render extra?.()}
    <IconButton onclick={onclose} aria-label="Close"><Icon name="x" /></IconButton>
  </header>
  {@render children()}
</div>

<style src="./LogBox.css"></style>
