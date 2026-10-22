<script lang="ts">
  import type { Side } from "../snapshot";
  let { side }: { side: Side } = $props();
</script>

{#if side === "home"}
  <span class="side-tag home">Home</span>
{:else}
  <span class="side-tag away">Away</span>
{/if}
