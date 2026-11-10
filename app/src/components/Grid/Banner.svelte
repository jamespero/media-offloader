<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import type { IconName } from "../../icons";
  import type { Snippet } from "svelte";

  /** `lead` is the opening sentence, in bold; the rest follows in the body text. */
  let {
    icon,
    tone = "",
    lead,
    action,
    children,
  }: { icon: IconName; tone?: "" | "info" | "bad"; lead: Snippet; action?: Snippet; children: Snippet } = $props();
</script>

<div class="banner">
  <div class={["ic", tone]}><Icon name={icon} /></div>
  <div class="tx"><b>{@render lead()}</b> {@render children()}</div>
  {@render action?.()}
</div>

<style src="./Banner.css"></style>
