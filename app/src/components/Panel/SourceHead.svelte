<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../Icon/Icon.svelte";
  import Row from "../Row/Row.svelte";
  import type { IconName } from "../../icons";
  import Heading from "./Heading.svelte";

  /**
   * The top of the source panel: what kind of source, its name, and a line about it.
   * The name is a button (with `extra` beside it, such as Eject) unless `still`, as in history.
   */
  let {
    icon,
    label,
    meta,
    path,
    onclick,
    chevron = false,
    still = false,
    extra,
  }: {
    icon: IconName;
    label: string;
    meta: string;
    path?: string;
    onclick?: (e: MouseEvent & { currentTarget: HTMLButtonElement }) => void;
    chevron?: boolean;
    still?: boolean;
    extra?: Snippet;
  } = $props();
</script>

<div class="srcicon"><Icon name={icon} /></div>
<Heading text="source" small />
{#if still}
  <div class="vol still"><span>{label}</span></div>
{:else}
  <Row gap="tighter">
    <button class="vol" {onclick} title={path}>
      <span>{label}</span>
      {#if chevron}<Icon name="chev" />{/if}
    </button>
    {@render extra?.()}
  </Row>
{/if}
<div class="meta">{meta}</div>

<style src="./SourceHead.css"></style>
