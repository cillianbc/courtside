<!-- The key bound to an action on this computer, as a keycap; nothing when
     the action has no key. Labels always come from the keymap. -->
<script lang="ts">
  import { keyFor } from "../settings";
  import { ui } from "../ui.svelte";

  let { action }: { action: string } = $props();
  const key = $derived(keyFor(ui.keymap, action));
</script>

{#if key}<kbd class="kbd">{key}</kbd>{/if}
