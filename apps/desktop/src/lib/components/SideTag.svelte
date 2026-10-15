<script lang="ts">
  import type { Side } from "../snapshot";
  let { side }: { side: Side } = $props();
</script>

{#if side === "home"}
  <span class="side-tag home">HOME</span>
{:else}
  <span class="side-tag away">AWAY</span>
{/if}
