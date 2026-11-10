<script lang="ts">
  import IconButton from "../IconButton/IconButton.svelte";
  import Button from "../Button/Button.svelte";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { ic } from "../../icons";
  import { reveal } from "../../places";
  import { fileManager } from "../../state.svelte.ts";
  import LogBox from "./LogBox.svelte";

  let { path, text }: { path: string; text: string } = $props();

  function tone(line: string) {
    if (/^OK /.test(line) || /^Result +All files verified/.test(line)) return "ok";
    if (/^(FAIL|UNREADABLE) /.test(line) || /^Result +(?!All)/.test(line)) return "er";
    if (/^(==|Media Offloader)/.test(line)) return "hd";
    return null;
  }

  const lines = $derived(text.split(/\r?\n/).map((l) => [l, tone(l)] as const));
</script>

<LogBox>
  <header>
    {@html ic.file()}<b>Transfer log</b>
    <Button size="sm" onclick={() => reveal(path)}>{@html ic.reveal(13)} Show in {fileManager}</Button>
    <IconButton onclick={closeOverlay} aria-label="Close">{@html ic.x()}</IconButton>
  </header>
  <pre>{#each lines as [line, cls], i}{#if i > 0}{"\n"}{/if}{#if cls}<span class={cls}>{line}</span>{:else}{line}{/if}{/each}</pre>
</LogBox>
