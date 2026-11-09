<script lang="ts">
  let {
    narrow = false,
    element = $bindable(),
    ...attrs
  }: {
    /** For a short number rather than a line of text. */
    narrow?: boolean;
    element?: HTMLInputElement;
    [attr: string]: unknown;
  } = $props();
</script>

<input class={["tin", narrow && "narrow"]} bind:this={element} {...attrs} />

<style src="./TextInput.css"></style>
