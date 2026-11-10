<script lang="ts">
  import type { HistoryItem } from "../../api";
  import { fmtSize } from "../../format";
  import { clock, resumeLabel } from "../../history";
  import { S } from "../../state.svelte.ts";
  import Banner from "../Grid/Banner.svelte";
  import Grid from "../Grid/Grid.svelte";

  let { item }: { item: HistoryItem } = $props();

  const when = $derived(`${new Date(item.started).toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" })} at ${clock(item.started)}`);
  const note = $derived(item.resumable ? ` ${resumeLabel(item)} continues in the same folder${item.roots.length > 1 ? "s" : ""}; files already verified aren't copied again.` : "");
  const verified = $derived(item.outcome?.kind === "verified");
</script>

<Banner icon={verified ? "shield" : "info"} tone={verified ? "" : item.outcome?.kind === "cancelled" ? "info" : "bad"}>
  <b>{item.label}</b>, offloaded {when}{item.runs > 1 ? ` in ${item.runs} runs` : ""} · {item.files} files, {fmtSize(item.bytes)}.{note}
</Banner>
<Grid files={S.histFiles} />
