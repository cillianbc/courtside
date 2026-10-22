<!-- Right-click menu on a code in the stream. -->
<script lang="ts">
  import { ui } from "../ui.svelte";
  export interface MenuItem {
    label: string;
    run: () => void;
    disabled?: boolean;
    danger?: boolean;
  }

  let {
    x,
    y,
    items,
    onclose,
    within,
  }: {
    x: number;
    y: number;
    items: MenuItem[];
    onclose: () => void;
    /** The panel the menu stays inside, so it keeps clear of the video. */
    within?: HTMLElement;
  } = $props();

  let box: HTMLDivElement | undefined = $state();
  // Kept inside its panel (else the window); the first item takes focus
  // for the keyboard.
  let left = $state(0);
  let top = $state(0);
  $effect(() => {
    if (!box) return;
    const r = box.getBoundingClientRect();
    const area = within?.getBoundingClientRect() ?? {
      left: 0,
      top: 0,
      right: window.innerWidth,
      bottom: window.innerHeight,
    };
    left = Math.max(area.left + 4, Math.min(x, area.right - r.width - 4));
    top = Math.max(area.top + 4, Math.min(y, area.bottom - r.height - 4));
    box.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
  });

  // The match video's native view would hide the menu where they overlap:
  // then the video steps aside while the menu is open.
  $effect(() => {
    if (!box) return;
    void left;
    void top;
    const m = box.getBoundingClientRect();
    const v = document.querySelector(".video .screen")?.getBoundingClientRect();
    const overlaps =
      !!v && m.left < v.right && m.right > v.left && m.top < v.bottom && m.bottom > v.top;
    if (!overlaps) return;
    ui.covering += 1;
    return () => (ui.covering -= 1);
  });

  // Run first: closing clears the caller's state (the code the menu was
  // opened on), which the item's action still reads.
  function pick(item: MenuItem) {
    item.run();
    onclose();
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
