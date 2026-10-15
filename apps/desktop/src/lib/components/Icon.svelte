<!-- Inline SVG icons (16×16, stroke), drawn for Courtside. -->
<script lang="ts">
  export type IconName =
    "timeout" | "sub" | "fwd" | "back" | "play" | "rewind" | "forward" | "swap" | "turn";
  let { name }: { name: IconName } = $props();
</script>

<svg
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "timeout"}
    <circle cx="8" cy="8.5" r="5.5" /><path d="M8 5.5v3l2 1.5M6.5 1.5h3" />
  {:else if name === "sub"}
    <path d="M5 13V3M2.5 5.5 5 3l2.5 2.5M11 3v10M8.5 10.5 11 13l2.5-2.5" />
  {:else if name === "fwd"}
    <path d="M13.5 8.5a5.5 5.5 0 1 1-1.6-3.9" /><path d="M12.5 1.5v3.5H9" />
  {:else if name === "back"}
    <path d="M2.5 8.5a5.5 5.5 0 1 0 1.6-3.9" /><path d="M3.5 1.5v3.5H7" />
  {:else if name === "play"}
    <path d="M5 3v10l8-5-8-5Z" />
  {:else if name === "rewind"}
    <path d="M8 4 3 8l5 4V4ZM14 4 9 8l5 4V4Z" />
  {:else if name === "forward"}
    <path d="M8 4l5 4-5 4V4ZM2 4l5 4-5 4V4Z" />
  {:else if name === "swap"}
    <path d="M2 5h10M9 2l3 3-3 3M14 11H4M7 8l-3 3 3 3" />
  {:else if name === "turn"}
    <rect x="1.5" y="6" width="8" height="8" rx="1" /><path
      d="M7 2.5h3a3.5 3.5 0 0 1 3.5 3.5v2.5"
    /><path d="M11.5 7l2 2 2-2" />
  {/if}
</svg>
