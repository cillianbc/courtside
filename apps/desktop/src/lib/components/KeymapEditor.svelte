<script lang="ts">
  import {
    ACTIONS,
    conflicts,
    effectiveKeymap,
    keyLabel,
    keyString,
    ALT_NAME,
    MOD_NAME,
    usableWhileTyping,
    type KeyBinding,
  } from "../settings";

  let { keymap = $bindable() }: { keymap: KeyBinding[] } = $props();

  let rows = $state<KeyBinding[]>(effectiveKeymap(keymap));
  let recording = $state<string | null>(null);
  const clash = $derived(conflicts(rows));

  $effect(() => {
    keymap = rows.map((r) => ({ ...r }));
  });

  function record(e: KeyboardEvent, action: string) {
    if (recording !== action) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = null;
      return;
    }
    const keys = keyString(e);
    if (!keys) return;
    const row = rows.find((r) => r.action === action);
    if (row) row.keys = keys;
    recording = null;
  }

  function reset() {
    rows = effectiveKeymap([]);
  }

  const label = (id: string) => ACTIONS.find((a) => a.id === id)?.label ?? id;
</script>

<!-- WebKit doesn't focus a clicked button, so the key is caught on the
     window while a row is recording. -->
<svelte:window
  onkeydowncapture={(e) => {
    if (recording !== null) record(e, recording);
  }}
/>

<table class="list">
  <thead><tr><th>Action</th><th>Keys</th><th></th></tr></thead>
  <tbody>
    {#each rows as r (r.action)}
      <tr>
        <td>{label(r.action)}</td>
        <td>
          <button
            type="button"
            class="key-cap"
            class:recording={recording === r.action}
            class:bad={clash.has(r.keys)}
            onclick={() => (recording = r.action)}
          >
            {recording === r.action ? "Press keys… (Esc cancels)" : keyLabel(r.keys) || "–"}
          </button>
        </td>
        <td class="sub-note" class:bad={clash.has(r.keys)}>
          {#if clash.has(r.keys)}
            Also used by another action.
          {:else if r.keys && !usableWhileTyping(r.keys)}
            Not while typing a code: add {MOD_NAME} or {ALT_NAME}.
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
<div class="actions">
  <button class="btn" type="button" onclick={reset}>Back to Courtside's keys</button>
</div>
