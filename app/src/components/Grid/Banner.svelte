<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import type { IconName } from "../../icons";
  import type { Snippet } from "svelte";

  let { icon, tone = "", action, children }: { icon: IconName; tone?: "" | "info" | "bad"; action?: Snippet; children: Snippet } = $props();
</script>

<div class="banner">
  <div class={["ic", tone]}><Icon name={icon} /></div>
  <div class="tx">{@render children()}</div>
  {@render action?.()}
</div>

<style src="./Banner.css"></style>
