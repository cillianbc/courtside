<!-- The Video layout's search, popped out beside the column while it's open
     (VolleyStation-style "Search results"): the find bar, then each code
     found with its set and score; a click picks it and seeks the video.
     "Play on video" hands the same filter to Clips and plays them. The
     video narrows while this is open; the picture is a native layer, so
     nothing can sit on top of it. -->
<script lang="ts">
  import { pickCode } from "../jump";
  import type { Row, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";
  import FindBar from "./FindBar.svelte";

  let { snap }: { snap: Snapshot } = $props();

  interface Hit {
    row: Row;
    set: number;
    /** The score when the code was played (before its rally's point). */
    score: string;
  }

  // Each code found, in match order, with the score its rally started at.
  const hits = $derived.by(() => {
    const want = new Set(ui.found);
    const out: Hit[] = [];
    const before: Record<number, string> = {};
    for (const g of snap.groups) {
      const at = before[g.set] ?? "0:0";
      for (const r of g.rows) if (want.has(r.id)) out.push({ row: r, set: g.set, score: at });
      if (g.scoreAfter) before[g.set] = `${g.scoreAfter.home}:${g.scoreAfter.visiting}`;
    }
    return out;
  });

  let list: HTMLOListElement | undefined = $state();
  // The picked code stays in view as Enter / ↑↓ step through the hits.
  $effect(() => {
    const id = ui.selected;
    if (id) list?.querySelector(`[data-hit="${id}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function playOnVideo() {
    ui.clipsFor(snap.matchId).filter = $state.snapshot(ui.findFilter);
    ui.clipsAutoplay = true;
    ui.closeFind();
    ui.openClips();
  }
</script>

<section class="panel search-panel" aria-label="Search results">
  <div class="search-head">
    <h2>Search results</h2>
    <button
      class="btn icon"
      type="button"
      aria-label="Close the search"
      title="Close the search (Esc)"
      onclick={() => {
        ui.closeFind();
        ui.focusCode += 1;
      }}>×</button
    >
  </div>
  <FindBar {snap} />
  {#if hits.length}
    <ol class="hits-list" bind:this={list}>
      {#each hits as h (h.row.id)}
        <li>
          <button
            type="button"
            class="hit {h.row.team === 'home' ? 'home' : h.row.team ? 'away' : ''}"
            class:picked={ui.selected === h.row.id}
            data-hit={h.row.id}
            title={h.row.meaning}
            onclick={() => pickCode(snap, h.row.id)}
            ><code>{h.row.text}</code><span class="where">S{h.set} {h.score}</span></button
          >
        </li>
      {/each}
    </ol>
  {:else}
    <p class="empty-note">
      {ui.findFilter.text || ui.found.length
        ? "No codes found."
        : "Type part of a code, or Filter."}
    </p>
  {/if}
  <button
    class="btn primary play-found"
    type="button"
    disabled={!hits.length || !video.open}
    title={video.open
      ? "Play these codes as clips (opens Clips with this search)"
      : "Open the match video first"}
    onclick={playOnVideo}>Play on video</button
  >
</section>
