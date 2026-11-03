<script lang="ts">
  import * as backend from "../backend";
  import { pickCode } from "../jump";
  import type { Rule, RuleChange, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  // Find and replace (#25): one change to every code the search finds,
  // previewed before it's made; one undo takes it back.
  let { snap }: { snap: Snapshot } = $props();

  let mode = $state<"replace" | "after" | "before">("replace");
  let text = $state("");
  let team = $state<"same" | "other">("other");
  let changes = $state<RuleChange[]>([]);
  let problem = $state<string | null>(null);
  let listOpen = $state(false);

  const rule = $derived<Rule>(
    mode === "replace"
      ? { kind: "replace", with: text }
      : { kind: "add", code: text, place: mode, team },
  );
  const ready = $derived(mode === "replace" ? !!ui.findFilter.text : !!text.trim());

  $effect(() => {
    void snap;
    const f = $state.snapshot(ui.findFilter);
    const r = $state.snapshot(rule);
    if (!ready) {
      changes = [];
      problem = null;
      return;
    }
    backend.previewRule(f, r).then(
      (c) => {
        changes = c;
        problem = null;
      },
      (e) => {
        changes = [];
        problem = String(e);
      },
    );
  });

  const done = $derived(changes.filter((c) => !c.error));
  const skipped = $derived(changes.length - done.length);
  const verb = $derived(mode === "replace" ? "Replace" : "Add");

  function apply() {
    const what =
      mode === "replace"
        ? `Replaced ${ui.findFilter.text} with ${text} in ${done.length} ${done.length === 1 ? "code" : "codes"}`
        : `Added ${done.length} ${done.length === 1 ? "code" : "codes"}`;
    void ui.apply(what, () => backend.applyRule($state.snapshot(ui.findFilter), rule), true);
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Enter" && done.length) {
      e.preventDefault();
      apply();
    } else if (e.key === "Escape") {
      e.preventDefault();
      ui.replacing = false;
    }
  }
</script>

<div class="replace-bar" role="group" aria-label="Find and replace">
  <div class="replace-row">
    <select bind:value={mode} aria-label="What to do with each code found">
      <option value="replace">Replace with</option>
      <option value="after">Add after</option>
      <option value="before">Add before</option>
    </select>
    <input
      bind:value={text}
      type="text"
      spellcheck="false"
      autocomplete="off"
      placeholder={mode === "replace" ? "Replacement text" : "Code without the team: $$D, 5D-"}
      aria-label={mode === "replace" ? "Replacement text" : "Code to add"}
      title={mode === "replace"
        ? "Replaces the Find text in every code found, as if you had modified each one."
        : "Added next to every touch found, for its team or the other one; $$ for no particular player."}
      onkeydown={key}
    />
    {#if mode !== "replace"}
      <select bind:value={team} aria-label="Which team the code is for">
        <option value="other">other team</option>
        <option value="same">same team</option>
      </select>
    {/if}
    <button
      class="btn small primary"
      type="button"
      disabled={!done.length}
      title="One undo takes it all back"
      onclick={apply}>{verb} {done.length || ""}</button
    >
  </div>
  {#if problem}
    <p class="replace-note bad">{problem}</p>
  {:else if mode === "replace" && !ui.findFilter.text}
    <p class="replace-note">Type the text to replace in Find.</p>
  {:else if changes.length}
    <button
      class="replace-note link"
      type="button"
      aria-expanded={listOpen}
      onclick={() => (listOpen = !listOpen)}
      >{done.length} to change{#if skipped}, {skipped} left as they are{/if} · {listOpen
        ? "hide"
        : "show"}</button
    >
    {#if listOpen}
      <ul class="replace-list">
        {#each changes as c (c.id)}
          <li class:bad={!!c.error}>
            <button type="button" onclick={() => pickCode(snap, c.id)}>
              <code>{c.code}</code>
              {#if c.error}<span>{c.error}</span>{:else}→ <code>{c.result.join(" ")}</code>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>
