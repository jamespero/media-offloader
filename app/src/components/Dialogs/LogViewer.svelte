<script lang="ts">
  import { ic } from "../../icons";
  import { fileManager } from "../../state.svelte.ts";

  let { path, text }: { path: string; text: string } = $props();

  function tone(line: string) {
    if (/^OK /.test(line) || /^Result +All files verified/.test(line)) return "ok";
    if (/^(FAIL|UNREADABLE) /.test(line) || /^Result +(?!All)/.test(line)) return "er";
    if (/^(==|Media Offloader)/.test(line)) return "hd";
    return null;
  }

  const lines = $derived(text.split(/\r?\n/).map((l) => [l, tone(l)] as const));
</script>

<div class="logbox">
  <header>
    {@html ic.file()}<b>Transfer log</b>
    <button class="btn sm" data-act="revealpath" data-path={path}>{@html ic.reveal(13)} Show in {fileManager}</button>
    <button class="iconbtn" data-act="closeov" aria-label="Close">{@html ic.x()}</button>
  </header>
  <pre>{#each lines as [line, cls], i}{#if i > 0}{"\n"}{/if}{#if cls}<span class={cls}>{line}</span>{:else}{line}{/if}{/each}</pre>
</div>
