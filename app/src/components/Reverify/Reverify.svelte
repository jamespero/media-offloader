<script lang="ts">
  import Icon from "../Icon/Icon.svelte";
  import Button from "../Button/Button.svelte";
  import type { CheckStatus, ManifestSource } from "../../api";
  import { fmtSize } from "../../format";
  import { closeOverlay } from "../Overlay/overlayState.svelte.ts";
  import { reveal } from "../../places";
  import { R, reverifyLog } from "../../reverify.svelte.ts";
  import { fileManager } from "../../state.svelte.ts";
  import LogBox from "../Dialogs/LogBox.svelte";
  import ProgressFill from "../ProgressFill/ProgressFill.svelte";

  function sourceText(s: ManifestSource | null) {
    if (!s) return "";
    return s.kind === "mhl" ? `ASC MHL history (${s.generations} generation${s.generations === 1 ? "" : "s"})` : s.file;
  }

  function statusText(s: CheckStatus): string {
    switch (s.state) {
      case "missing":
        return "missing";
      case "sizeDiffers":
        return `size is ${s.actual} bytes, the manifest says ${s.expected}`;
      case "mismatch":
        return `${s.format} doesn't match (${s.actual.slice(0, 16)}…, expected ${s.expected.slice(0, 16)}…)`;
      case "readError":
        return `couldn't read: ${s.message}`;
      default:
        return "";
    }
  }

  const pct = $derived(R.bytes ? (R.done / R.bytes) * 100 : 0);
  const report = $derived(R.report);
  const bad = $derived(report ? report.files.filter((f) => f.status.state !== "verified" && f.status.state !== "pending") : []);
</script>

<LogBox fit icon="shield" title="Re-verify" onclose={closeOverlay}>
  {#snippet extra()}<span class="rvpath" title={R.path}>{R.path}</span>{/snippet}
  <div class="rvbody" id="rvbody">
    {#if R.error}
      <div class="rvres failed"><Icon name="alert" size={18} /><span>{R.error}</span></div>
      <div class="acts"><Button onclick={closeOverlay}>Close</Button></div>
    {:else if R.running || !report}
      <div class="rvbar"><ProgressFill {pct} /></div>
      <div class="rvstat"><span class="pct-s">{Math.floor(pct)}%</span><span class="f">{R.current}</span></div>
      <div class="rvnote">
        {#if R.files}{R.checked} of {R.files} files · {fmtSize(R.bytes)} · checking against {sourceText(R.source)}{:else}Reading the manifest…{/if}
        {#if R.failed}· <span class="problem">{R.failed} not matching so far</span>{/if}
      </div>
      <div class="acts"><Button onclick={closeOverlay}>Stop</Button></div>
    {:else}
      {#if report.outcome.kind === "verified"}
        <div class="rvres ok"><Icon name="check" size={18} stroke={2.6} /><span>All {report.files.length} files match their recorded checksums.</span></div>
      {:else if report.outcome.kind === "failed"}
        <div class="rvres failed"><Icon name="alert" size={18} /><span>{report.outcome.files === 1 ? "1 file doesn't" : `${report.outcome.files} files don't`} match their recorded checksums.</span></div>
      {:else}
        <div class="rvres"><Icon name="info" size={18} /><span>Stopped after {R.checked} of {report.files.length} files.</span></div>
      {/if}
      {#if report.manifestProblems.length || bad.length || report.notListed.length}
        <!-- This list keeps its whitespace, so everything stays on one line. -->
        <div class="rvlist">{#each report.manifestProblems as p}<div class="er">{`Manifest: ${p}`}</div>{/each}{#each bad as f}<div class="er">{`FAIL  ${f.path} — ${statusText(f.status)}`}</div>{/each}{#each report.notListed.slice(0, 200) as p}<div>{`new   ${p} — not in any manifest`}</div>{/each}</div>
      {/if}
      <div class="rvnote">Checked against {sourceText(report.source)} · every file read from disk, bypassing the OS cache.</div>
      <div class="acts">
        {#if report.logPath}<Button onclick={reverifyLog}><Icon name="file" size={13} /> Show log</Button>{/if}
        <Button onclick={() => reveal(report.root)}><Icon name="reveal" size={13} /> Show in {fileManager}</Button>
        <Button onclick={closeOverlay}>Close</Button>
      </div>
    {/if}
  </div>
</LogBox>

<style src="./Reverify.css"></style>
