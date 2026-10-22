<script lang="ts">
  import * as backend from "../backend";
  import { hintFor } from "../hint";
  import { keyFor, keyString, textFor, typeText } from "../settings";
  import type { Preview, Side } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Kbd from "./Kbd.svelte";

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

  // The set just won, for the confirm line; "Not yet" hides it until the
  // score changes.
  let notYet = $state<string | null>(null);
  const setEnd = $derived.by(() => {
    if (!snap?.setEndDue || ui.editing) return null;
    const s = snap.score;
    const winner = s.home > s.visiting ? snap.teams.home.name : snap.teams.visiting.name;
    const key = `${snap.set}-${s.home}:${s.visiting}`;
    return key === notYet
      ? null
      : { key, text: `Set ${snap.set} won ${s.home}:${s.visiting} by ${winner}. End set?` };
  });
  // The earlier code being changed, by its match-wide number (v6 T4).
  const editNumber = $derived.by(() => {
    const id = ui.editing?.id;
    if (!id || !snap) return null;
    for (const g of snap.groups) {
      const r = g.rows.find((x) => x.id === id);
      if (r) return r.index + 1;
    }
    return null;
  });

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
      if (setEnd && !ui.code) notYet = setEnd.key;
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

<!-- v6 T7/T9/T10: an optional line (set end, or the code being changed),
     the input between the point buttons, and one fixed read-back line, so the
     bar never changes height while the scout types. -->
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
      >Point<span class="pt-name">&nbsp;{snap?.teams[side].name ?? ""}</span><Kbd
        action={left ? "pointLeft" : "pointRight"}
      /></button
    >
  {/snippet}
  {#if setEnd}
    <div class="confirm">
      <span class="txt">{setEnd.text}</span>
      <button
        type="button"
        class="btn primary"
        title="End the set (Enter on an empty line)"
        onclick={() => void ui.endSet()}>End set <kbd class="kbd">↵</kbd></button
      >
      <button type="button" class="btn" onclick={() => (notYet = setEnd.key)}
        >Not yet <kbd class="kbd">Esc</kbd></button
      >
    </div>
  {:else if ui.editing}
    <div class="editing-tag">
      {{
        correct: "Modifying",
        insert: "Adding before",
        after: "Adding after",
        rally: "New rally before",
      }[ui.editing.mode]}
      {#if editNumber}code {editNumber}:{/if} <code>{ui.editing.text}</code> · Esc cancels
    </div>
  {/if}
  <div class="cmd-row">
    {@render point(ui.order[0] ?? "home", true)}
    <div class="field">
      <span class="prompt" aria-hidden="true">›</span>
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
    </div>
    {@render point(ui.order[1] ?? "visiting", false)}
  </div>
  <output class="hint" id="hint" title={hint.text}>{hint.text}</output>
</section>
