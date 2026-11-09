<script lang="ts">
  let { path }: { path: string } = $props();
</script>

<!-- rtl + bdi keeps the end of a long path (the interesting part) visible. -->
<div class="path" title={path}><bdi>{path}</bdi></div>
