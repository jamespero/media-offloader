<script lang="ts">
  import { S, fileManager, running } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  const isMac = navigator.userAgent.includes("Mac");
  const mod = isMac ? "⌘" : "Ctrl+";

  const copyRoot = $derived(S.phase === "done" || S.phase === "error" ? S.dests?.[0]?.root : null);
</script>

<MenuItem icon="newf" label="New Offload" act="new" disabled={running()} shortcut="{mod}N" />
<MenuItem icon="refresh" label="Re-verify a Folder…" act="reverify" shortcut="{mod}R" />
<div class="sep"></div>
<MenuItem icon="file" label="Open Log for This Offload" act="log" disabled={!S.report?.logPath} shortcut="{mod}L" />
<MenuItem icon="folder" label="Show Logs Folder" act="logsfolder" />
<MenuItem icon="reveal" label="{isMac ? 'Reveal Copy in' : 'Show Copy in'} {fileManager}" act="revealcopy" disabled={!copyRoot} />
<div class="sep"></div>
<MenuItem icon="gear" label="Settings…" act="settings" shortcut="{mod}," />
<MenuItem icon="info" label="About Media Offloader" act="about" />
