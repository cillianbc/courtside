<!-- Keys that type other text on the command line (`]` types `#`). -->
<script lang="ts">
  import { keyLabel, keyString, type KeyBinding, type KeyText } from "../settings";

  let { keyText = $bindable(), keymap }: { keyText: KeyText[]; keymap: KeyBinding[] } = $props();

  let recording = $state<number | null>(null);

  // A key set twice here, or also an action's key.
  function clash(i: number): string {
    const k = keyText[i]?.keys;
    if (!k) return "";
    if (keyText.some((o, j) => j !== i && o.keys === k)) return "Set twice.";
    if (keymap.some((b) => b.keys === k)) return "Also an action's key.";
    return "";
  }

  function record(e: KeyboardEvent, i: number) {
    if (recording !== i) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = null;
      return;
    }
    const keys = keyString(e);
    const row = keyText[i];
    if (!keys || !row) return;
    row.keys = keys;
    recording = null;
  }

  function add() {
    keyText.push({ keys: "", text: "" });
    recording = keyText.length - 1;
  }
</script>

<!-- WebKit doesn't focus a clicked button, so the key is caught on the
     window while a row is recording. -->
<svelte:window
  onkeydowncapture={(e) => {
    if (recording !== null) record(e, recording);
  }}
/>

<table class="list">
  <thead><tr><th>Key</th><th>Types</th><th></th><th></th></tr></thead>
  <tbody>
    {#each keyText as row, i (i)}
      <tr>
        <td>
          <button
            type="button"
            class="key-cap"
            class:recording={recording === i}
            class:bad={!!clash(i)}
            onclick={() => (recording = i)}
          >
            {recording === i ? "Press a key… (Esc cancels)" : keyLabel(row.keys) || "–"}
          </button>
        </td>
        <td>
          <input
            class="short mono"
            maxlength="20"
            bind:value={row.text}
            aria-label={`What ${keyLabel(row.keys) || "the key"} types`}
          />
        </td>
        <td class="sub-note" class:bad={!!clash(i)}>{clash(i)}</td>
        <td>
          <button class="btn" type="button" onclick={() => keyText.splice(i, 1)}>Remove</button>
        </td>
      </tr>
    {:else}
      <tr><td colspan="4" class="sub-note">No keys remapped.</td></tr>
    {/each}
  </tbody>
</table>
<div class="actions">
  <button class="btn" type="button" onclick={add}>Add a key</button>
</div>
