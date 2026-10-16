<!-- Right-click menu on a code in the stream. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { ui } from "../ui.svelte";
  export interface MenuItem {
    label: string;
    run: () => void;
    disabled?: boolean;
    danger?: boolean;
  }

  let { x, y, items, onclose }: { x: number; y: number; items: MenuItem[]; onclose: () => void } =
    $props();

  let box: HTMLDivElement | undefined = $state();
  // Kept inside the window; the first item takes focus for the keyboard.
  let left = $state(0);
  let top = $state(0);
  $effect(() => {
    if (!box) return;
    const r = box.getBoundingClientRect();
    left = Math.max(4, Math.min(x, window.innerWidth - r.width - 4));
    top = Math.max(4, Math.min(y, window.innerHeight - r.height - 4));
    box.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
  });

  // The match video's native view would hide the menu.
  onMount(() => {
    ui.covering += 1;
    return () => (ui.covering -= 1);
  });

  function pick(item: MenuItem) {
    onclose();
    item.run();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const buttons = [
        ...(box?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []),
      ];
      const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const step = e.key === "ArrowDown" ? 1 : -1;
      buttons[(i + step + buttons.length) % buttons.length]?.focus();
    }
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (box && !box.contains(e.target as Node)) onclose();
  }}
  onblur={onclose}
  onresize={onclose}
/>

<div
  class="code-menu"
  role="menu"
  tabindex="-1"
  bind:this={box}
  style:left="{left}px"
  style:top="{top}px"
  {onkeydown}
>
  {#each items as item (item.label)}
    <button
      type="button"
      role="menuitem"
      class:danger={item.danger}
      disabled={item.disabled}
      onclick={() => pick(item)}>{item.label}</button
    >
  {/each}
</div>
