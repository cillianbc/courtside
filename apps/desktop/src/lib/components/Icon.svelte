<!-- Inline SVG icons (16×16, stroke), drawn for Courtside. -->
<script lang="ts">
  export type IconName =
    | "timeout"
    | "sub"
    | "fwd"
    | "back"
    | "play"
    | "rewind"
    | "forward"
    | "swap"
    | "turn"
    | "lineup"
    | "pause"
    | "stepBack"
    | "stepFwd"
    | "arrow"
    | "menu"
    | "export"
    | "sun"
    | "moon"
    | "system"
    | "boost"
    | "undo"
    | "check"
    | "left"
    | "sync"
    | "clips"
    | "open"
    | "find";
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
  {#if name === "menu"}
    <path d="M10 3 5 8l5 5" />
  {:else if name === "export"}
    <path d="M8 2v8M5 7l3 3 3-3M3 12v2h10v-2" />
  {:else if name === "sun"}
    <circle cx="8" cy="8" r="3" /><path
      d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1"
    />
  {:else if name === "moon"}
    <path d="M13 9.5A5.5 5.5 0 0 1 6.5 3a5.5 5.5 0 1 0 6.5 6.5Z" />
  {:else if name === "system"}
    <rect x="2" y="3" width="12" height="8" rx="1" /><path d="M6 14h4M8 11v3" />
  {:else if name === "boost"}
    <circle cx="8" cy="8" r="5.5" /><path d="M8 2.5v11A5.5 5.5 0 0 0 8 2.5Z" fill="currentColor" />
  {:else if name === "undo"}
    <path d="M5 3 2 6l3 3M2 6h7a4 4 0 0 1 0 8H7" />
  {:else if name === "check"}
    <path d="m3 8.5 3 3 7-7" />
  {:else if name === "left"}
    <path d="M13 8H3M7 4 3 8l4 4" />
  {:else if name === "sync"}
    <path d="M3 7a5 5 0 0 1 9-2M13 9a5 5 0 0 1-9 2M12 2v3H9M4 14v-3h3" />
  {:else if name === "clips"}
    <rect x="2" y="3" width="12" height="10" rx="1" /><path d="M2 6h12M5 3l1 3M9 3l1 3" />
  {:else if name === "find"}
    <circle cx="7" cy="7" r="4.5" /><path d="m10.5 10.5 3.5 3.5" />
  {:else if name === "open"}
    <path d="M2 4h4l1.5 1.5H14V13H2Z" />
  {:else if name === "arrow"}
    <rect x="1.5" y="3" width="13" height="10" rx="1" /><path d="M4 11l7-5M8.5 6H11v2.5" />
  {:else if name === "lineup"}
    <rect x="2" y="3" width="12" height="10" rx="1" /><path d="M2 8h12M6 3v10M10 3v10" />
  {:else if name === "timeout"}
    <circle cx="8" cy="8.5" r="5.5" /><path d="M8 5.5v3l2 1.5M6.5 1.5h3" />
  {:else if name === "sub"}
    <path d="M5 13V3M2.5 5.5 5 3l2.5 2.5M11 3v10M8.5 10.5 11 13l2.5-2.5" />
  {:else if name === "fwd"}
    <path d="M13.5 8.5a5.5 5.5 0 1 1-1.6-3.9" /><path d="M12.5 1.5v3.5H9" />
  {:else if name === "back"}
    <path d="M2.5 8.5a5.5 5.5 0 1 0 1.6-3.9" /><path d="M3.5 1.5v3.5H7" />
  {:else if name === "pause"}
    <path d="M5 3v10M11 3v10" />
  {:else if name === "stepBack"}
    <path d="M4 3v10M13 3 6 8l7 5V3Z" />
  {:else if name === "stepFwd"}
    <path d="M12 3v10M3 3l7 5-7 5V3Z" />
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
