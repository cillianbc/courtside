<script lang="ts">
  import * as backend from "../backend";
  import { hintFor, type Hint } from "../hint";
  import type { Preview } from "../snapshot";
  import { ui } from "../ui.svelte";

  let input: HTMLInputElement | undefined = $state();
  let preview = $state<Preview | null>(null);
  // The result of the last Enter, shown until the scout types again.
  let result = $state<Hint | null>(null);
  let busy = false;

  $effect(() => {
    void ui.focusCode;
    input?.focus();
  });

  // Read back as the scout types; ignore answers to older text.
  $effect(() => {
    const text = ui.code;
    if (!text.trim()) {
      preview = null;
      return;
    }
    result = null;
    backend.preview(text).then(
      (p) => {
        if (ui.code === text) preview = p;
      },
      () => {},
    );
  });

  const hint = $derived(result ?? hintFor(ui.code, preview));

  async function commit() {
    const text = ui.code.trim();
    if (busy) return;
    if (!text) {
      result = { text: "Type a code first.", bad: true };
      return;
    }
    busy = true;
    try {
      ui.snapshot = await backend.commit(text);
      ui.code = "";
      ui.shownSet = null;
      result = { text: `Added ${text}.`, bad: false };
    } catch (e) {
      result = { text: String(e), bad: true };
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      ui.code = "";
      result = null;
    } else if (e.key === "Enter") {
      e.preventDefault();
      void commit();
    }
  }
</script>

<section class="cmd" class:bad={hint.bad} aria-label="Command bar">
  <span class="prompt" aria-hidden="true">›</span>
  <input
    class="code-input"
    bind:this={input}
    bind:value={ui.code}
    {onkeydown}
    autocomplete="off"
    spellcheck="false"
    aria-label="Scout code"
    aria-describedby="hint"
    placeholder="a08RM+"
  />
  <output class="hint" id="hint">{hint.text}</output>
</section>
