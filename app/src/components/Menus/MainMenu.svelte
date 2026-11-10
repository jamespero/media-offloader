<script lang="ts">
  import { settings } from "../Settings/settingsState.svelte.ts";
  import { about, openRunLog, reverifyIntro } from "../../dialogs";
  import { newOffload } from "../../offload";
  import { revealCopy, showLogsFolder } from "../../places";
  import { S, fileManager, running } from "../../state.svelte.ts";
  import MenuItem from "../Menu/MenuItem.svelte";

  const isMac = navigator.userAgent.includes("Mac");
  const mod = isMac ? "⌘" : "Ctrl+";

  const copyRoot = $derived(S.phase === "done" || S.phase === "error" ? S.dests?.[0]?.root : null);
</script>

<MenuItem icon="newf" label="New Offload" onclick={newOffload} disabled={running()} shortcut="{mod}N" />
<MenuItem icon="refresh" label="Re-verify a Folder…" onclick={reverifyIntro} shortcut="{mod}R" />
<div class="sep"></div>
<MenuItem icon="file" label="Open Log for This Offload" onclick={openRunLog} disabled={!S.report?.logPath} shortcut="{mod}L" />
<MenuItem icon="folder" label="Show Logs Folder" onclick={showLogsFolder} />
<MenuItem icon="reveal" label="{isMac ? 'Reveal Copy in' : 'Show Copy in'} {fileManager}" onclick={revealCopy} disabled={!copyRoot} />
<div class="sep"></div>
<MenuItem icon="gear" label="Settings…" onclick={() => settings()} shortcut="{mod}," />
<MenuItem icon="info" label="About Media Offloader" onclick={about} />
