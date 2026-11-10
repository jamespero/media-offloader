<script lang="ts">
  /** A panel's heading, in lowercase. `small` for the narrower source panel. */
  let { text, small = false }: { text: string; small?: boolean } = $props();
</script>

<h2 class={{ small }}>{text}</h2>

<style src="./Heading.css"></style>
