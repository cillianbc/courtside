<!-- The code log's search (#25): text, Filter, Replace, stepping. In the
     code log (Coding layout) or the Video layout's pop-out SearchPanel; it
     runs the search, so only one is shown at a time. -->
<script lang="ts">
  import { stepFound } from "../actions";
  import * as backend from "../backend";
  import { filterSummary, hasFields } from "../filter";
  import type { Snapshot } from "../snapshot";
  import { withKey } from "../kbd";
  import { ui } from "../ui.svelte";
  import CodeFilter from "./CodeFilter.svelte";
  import ReplaceBar from "./ReplaceBar.svelte";

  let { snap }: { snap: Snapshot } = $props();

  // Search (#25): the codes the filter finds are marked; Enter and
  // Shift+Enter (or the previous/next play keys) step through them.
  let findInput: HTMLInputElement | undefined = $state();
  let findFields = $state(false);
  $effect(() => {
    if (!ui.finding) return;
    void snap;
    const f = $state.snapshot(ui.findFilter);
    backend.findCodes(f).then(
      (ids) => (ui.found = ids),
      () => (ui.found = []),
    );
  });
  $effect(() => {
    void ui.findFocus;
    if (ui.finding) findInput?.focus();
  });
  const asked = $derived(!!ui.findFilter.text || hasFields(ui.findFilter));
  const foundAt = $derived(ui.selected ? ui.found.indexOf(ui.selected) : -1);
  function findKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      stepFound(e.shiftKey ? -1 : 1);
    } else if (e.key === "Escape") {
      e.preventDefault();
      ui.closeFind();
      ui.focusCode += 1;
    }
  }
</script>

<div class="find-bar" role="search" aria-label="Search the codes">
  <input
    bind:this={findInput}
    bind:value={ui.findFilter.text}
    type="text"
    spellcheck="false"
    autocomplete="off"
    placeholder="Find: DT=, a12, *T"
    aria-label="Code text to find"
    title="Codes containing this text, as written. Enter: next, Shift+Enter: previous, Esc: close."
    onkeydown={findKey}
  />
  <span class="find-count" aria-live="polite"
    >{#if ui.found.length}{foundAt >= 0 ? `${foundAt + 1} of ` : ""}{ui.found
        .length}{:else if asked}None{/if}</span
  >
  <button
    class="btn icon"
    type="button"
    aria-label="Previous match"
    title={withKey("Previous match", "codePrev")}
    disabled={!ui.found.length}
    onclick={() => stepFound(-1)}>↑</button
  >
  <button
    class="btn icon"
    type="button"
    aria-label="Next match"
    title={withKey("Next match", "codeNext")}
    disabled={!ui.found.length}
    onclick={() => stepFound(1)}>↓</button
  >
  <button
    class="btn small"
    type="button"
    aria-expanded={findFields}
    aria-pressed={hasFields(ui.findFilter)}
    title={hasFields(ui.findFilter)
      ? filterSummary(ui.findFilter, snap).slice(0, -1).join(" · ")
      : "Find by team, player, skill, evaluation, rotation, phase or set"}
    onclick={() => (findFields = !findFields)}>Filter</button
  >
  <button
    class="btn small"
    type="button"
    aria-expanded={ui.replacing}
    title="Change every code found: replace text in it, or add a code before or after it"
    onclick={() => (ui.replacing = !ui.replacing)}>Replace</button
  >
  <button
    class="btn icon"
    type="button"
    aria-label="Close the search"
    onclick={() => ui.closeFind()}>×</button
  >
</div>
{#if findFields}
  <CodeFilter
    {snap}
    bind:filter={ui.findFilter}
    withText={false}
    ondone={() => (findFields = false)}
  />
{/if}
{#if ui.replacing}
  <ReplaceBar {snap} />
{/if}
