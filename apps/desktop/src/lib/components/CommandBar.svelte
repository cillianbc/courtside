<script lang="ts">
  import * as backend from "../backend";
  import { hintFor } from "../hint";
  import { keyFor, keyString, textFor, typeText } from "../settings";
  import type { Preview, Side } from "../snapshot";
  import { ui } from "../ui.svelte";

  let input: HTMLInputElement | undefined = $state();
  let preview = $state<Preview | null>(null);
  let busy = false;

  $effect(() => {
    void ui.focusCode;
    input?.focus();
  });

  // Focus that comes back by anything but a click in the box (a button,
  // Mod+K, the window) puts the caret at the end: the browser would put it
  // back where it last was, inside the codes already typed.
  let clicked = false;
  function onfocus() {
    if (clicked || !input) return;
    const end = input.value.length;
    input.setSelectionRange(end, end);
  }

  // Read back as the scout types; ignore answers to older text.
  $effect(() => {
    const text = ui.code;
    if (!text.trim()) {
      preview = null;
      return;
    }
    ui.result = null;
    backend.preview(text, ui.insertingBefore()).then(
      (p) => {
        if (ui.code === text) preview = p;
      },
      () => {},
    );
  });

  const hint = $derived(ui.result ?? hintFor(ui.code, preview));

  // At a rally's start the serve comes from the rotation (GitHub #12):
  // the empty line says who serves.
  const nextServe = $derived.by(() => {
    const snap = ui.snapshot;
    const open = snap?.groups.at(-1);
    if (!snap?.serving || ui.editing || !open || open.closed) return null;
    if (open.rows.some((r) => r.kind === "touch")) return null;
    const n = snap.court[snap.serving][0];
    return n == null ? null : `${snap.serving === "home" ? "*" : "a"}${String(n).padStart(2, "0")}`;
  });

  // Point buttons sit on the side of the team they score for, as the
  // court is drawn (they follow a side swap).
  const snap = $derived(ui.snapshot);
  const canPoint = $derived(
    !!snap &&
      !snap.matchOver &&
      !snap.setEndDue &&
      !snap.needsLineup.home &&
      !snap.needsLineup.visiting,
  );
  const pointLabel = (side: Side) => `Point to ${snap?.teams[side].name ?? side}`;

  async function commit() {
    const text = ui.code.trim();
    if (busy) return;
    if (!text && ui.snapshot?.setEndDue && !ui.editing) {
      busy = true;
      await ui.endSet();
      busy = false;
      return;
    }
    if (!text) {
      ui.result = { text: "Type a code first.", bad: true };
      return;
    }
    busy = true;
    if (await ui.submit(text)) ui.code = "";
    busy = false;
  }

  function onkeydown(e: KeyboardEvent) {
    // A remapped key types its text (Settings → Keys that type).
    const text = textFor(ui.keyText, keyString(e));
    if (text && input) {
      e.preventDefault();
      const t = typeText(
        ui.code,
        input.selectionStart ?? ui.code.length,
        input.selectionEnd ?? ui.code.length,
        text,
      );
      ui.code = t.value;
      input.value = t.value;
      input.setSelectionRange(t.caret, t.caret);
      ui.keyed();
      return;
    }
    if (e.key === "Escape") {
      if (ui.editing) ui.cancelEdit();
      ui.code = "";
      ui.clearStamps();
      ui.result = null;
    } else if (e.key === "Enter") {
      e.preventDefault();
      void commit();
    }
  }
</script>

<section class="cmd" class:bad={hint.bad} aria-label="Command bar">
  {#snippet point(side: Side, left: boolean)}
    <button
      type="button"
      class="point-btn"
      class:home={side === "home"}
      class:away={side === "visiting"}
      title="{pointLabel(side)} ({keyFor(ui.keymap, left ? 'pointLeft' : 'pointRight')})"
      disabled={!canPoint}
      onclick={() => void ui.point(side)}
      >{#if left}◀
      {/if}Point {snap?.teams[side].name ?? ""}{#if !left}&nbsp;▶{/if}</button
    >
  {/snippet}
  {@render point(ui.order[0] ?? "home", true)}
  {#if ui.editing}
    <span class="edit-tag"
      >{{ correct: "Modify", insert: "Before", after: "After", rally: "New rally before" }[
        ui.editing.mode
      ]}
      <code>{ui.editing.text}</code></span
    >
  {:else}
    <span class="prompt" aria-hidden="true">›</span>
  {/if}
  <input
    class="code-input"
    bind:this={input}
    bind:value={ui.code}
    {onkeydown}
    {onfocus}
    onpointerdown={() => {
      clicked = true;
      requestAnimationFrame(() => (clicked = false));
    }}
    oninput={() => ui.keyed()}
    autocomplete="off"
    spellcheck="false"
    aria-label="Scout code"
    aria-describedby="hint"
    placeholder={nextServe ? `${nextServe} serves: type + or the passer` : "a08RM+"}
  />
  <output class="hint" id="hint">{hint.text}</output>
  {#if snap?.setEndDue}
    <button
      type="button"
      class="btn primary"
      title="The score wins the set; end it (Enter on an empty line)"
      onclick={() => void ui.endSet()}>End set {snap.set}</button
    >
  {/if}
  {@render point(ui.order[1] ?? "visiting", false)}
</section>
