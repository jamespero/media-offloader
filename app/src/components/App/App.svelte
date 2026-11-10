<script lang="ts">
  import { onShortcut } from "../../shortcuts";
  import { S } from "../../state.svelte.ts";
  import BackupPanel from "../BackupPanel/BackupPanel.svelte";
  import CopyPanel from "../CopyPanel/CopyPanel.svelte";
  import Footer from "../Footer/Footer.svelte";
  import CurrentGrid from "../Grid/CurrentGrid.svelte";
  import EmptyState from "../Grid/EmptyState.svelte";
  import HistoryDestPanel from "../History/HistoryDestPanel.svelte";
  import HistoryGrid from "../History/HistoryGrid.svelte";
  import HistorySourcePanel from "../History/HistorySourcePanel.svelte";
  import Menu from "../Menu/Menu.svelte";
  import Overlay from "../Overlay/Overlay.svelte";
  import SourcePanel from "../SourcePanel/SourcePanel.svelte";
  import Strip from "../Strip/Strip.svelte";
  import Toast from "../Toast/Toast.svelte";
  import Tooltip from "../Tooltip/Tooltip.svelte";

  /** The past offload being shown, once its journal has been read. */
  const past = $derived(S.view ? (S.detail?.item ?? null) : null);
</script>

<svelte:window onkeydown={onShortcut} />

<div class="app">
  <div class="band">
    {#if past}
      <HistorySourcePanel item={past} />
      <HistoryDestPanel id="pCopy" item={past} dest={0} />
      <HistoryDestPanel id="pBak" item={past} dest={1} />
    {:else}
      <SourcePanel />
      <CopyPanel />
      <BackupPanel />
    {/if}
  </div>
  <Strip />
  <div class="gridwrap" id="gridwrap">
    {#if past}
      <HistoryGrid item={past} />
    {:else if S.view}
      <EmptyState title="Loading…" />
    {:else}
      <CurrentGrid />
    {/if}
  </div>
  <Footer />
  <Overlay />
  <Toast />
  <Tooltip />
  <Menu />
</div>

<style src="./App.css"></style>
