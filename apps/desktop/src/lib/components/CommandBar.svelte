<script lang="ts">
  import { ui } from "../ui.svelte";

  const IDLE = "Enter adds the code · Esc clears";

  let input: HTMLInputElement | undefined = $state();
  $effect(() => {
    void ui.focusCode;
    input?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      ui.code = "";
      ui.message = "";
    } else if (e.key === "Enter") {
      e.preventDefault();
      // Parsing and committing arrive with the core loop (step C.2).
      ui.message = "Coding isn't wired up yet.";
    }
  }
</script>

<section class="cmd" aria-label="Command bar">
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
  <output class="hint" id="hint">{ui.message || IDLE}</output>
</section>
