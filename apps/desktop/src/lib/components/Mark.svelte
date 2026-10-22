<!-- Severity marks (v6 T2): error a filled octagon, warning an outlined
     triangle, info an outlined "i". Shape, not just colour, tells them apart. -->
<script lang="ts">
  import type { Severity } from "../snapshot";
  let { severity, label }: { severity: Severity; label?: string } = $props();
  const name = { error: "Error", warning: "Warning", info: "Info" } as const;
</script>

<svg class="mark" viewBox="0 0 16 16" role="img" aria-label={label ?? name[severity]}>
  {#if severity === "error"}
    <polygon points="5.2,1 10.8,1 15,5.2 15,10.8 10.8,15 5.2,15 1,10.8 1,5.2" fill="var(--err)" />
    <path d="M8 4.2v4.6" stroke="var(--panel)" stroke-width="1.8" stroke-linecap="round" />
    <circle cx="8" cy="11.4" r="1.05" fill="var(--panel)" />
  {:else if severity === "warning"}
    <path
      d="M8 1.6 15 14.2H1Z"
      fill="var(--warn-mark)"
      stroke="var(--on-warn-mark)"
      stroke-width="1.2"
      stroke-linejoin="round"
    />
    <path d="M8 6v3.8" stroke="var(--on-warn-mark)" stroke-width="1.6" stroke-linecap="round" />
    <circle cx="8" cy="12" r=".95" fill="var(--on-warn-mark)" />
  {:else}
    <circle cx="8" cy="8" r="6.3" fill="none" stroke="var(--info)" stroke-width="1.5" />
    <circle cx="8" cy="4.9" r="1" fill="var(--info)" />
    <path d="M8 7.2v4.4" stroke="var(--info)" stroke-width="1.6" stroke-linecap="round" />
  {/if}
</svg>
