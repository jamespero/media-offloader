<script lang="ts">
  let {
    narrow = false,
    element = $bindable(),
    value = $bindable(),
    ...attrs
  }: {
    /** For a short number rather than a line of text. */
    narrow?: boolean;
    element?: HTMLInputElement;
    value?: string | number;
    [attr: string]: unknown;
  } = $props();
</script>

<input class={["tin", narrow && "narrow"]} bind:this={element} bind:value {...attrs} />

<style src="./TextInput.css"></style>
