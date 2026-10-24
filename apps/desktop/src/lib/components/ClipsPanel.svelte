<script lang="ts">
  import { onDestroy } from "svelte";
  import * as backend from "../backend";
  import { asRally, rallySpans, wholeRallies as wholeRallyClips } from "../clips";
  import { emptyFilter, filterSummary } from "../filter";
  import { withKey } from "../kbd";
  import { keyFor } from "../settings";
  import type { Clip, ClipFilter, Clips, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";
  import CodeFilter from "./CodeFilter.svelte";

  let { snap }: { snap: Snapshot } = $props();

  // The playlist the filter picks (PRD F-23), played clip by clip in the
  // video with the season's pre- and post-roll.
  let filter = $state<ClipFilter>(emptyFilter());
  let result = $state<Clips>({ clips: [], withoutVideo: 0 });
  let error = $state("");
  // Review and fix (#25): the clip being modified keeps the list's place
  // when the edit comes back; clips the scout has looked at are ticked.
  let kept: number | null = null;
  let checked = $state<Record<string, boolean>>({});

  $effect(() => {
    void snap;
    const f = $state.snapshot(filter);
    backend.clips(f).then(
      (c) => {
        result = c;
        error = "";
        if (kept !== null) {
          // Back from modifying a clip's code: stay where the scout was
          // (the next clip, when the edit took it out of the list).
          video.playlist = windows(c.clips);
          video.clipAt = Math.min(kept, c.clips.length - 1);
          kept = null;
          return;
        }
        // A new list: the clip playing is no longer where it was.
        video.stopClips();
        video.playlist = windows(c.clips);
      },
      (e) => (error = String(e)),
    );
  });

  // The clip playing is the picked code, so the court and the picked-code
  // keys (Modify, Alt+E) work on it.
  $effect(() => {
    const c = video.clip;
    if (c) ui.selected = c.id;
  });

  // A clip's whole rally (the code times don't always line up with the
  // touch); "Whole rallies" plays every clip that way, each rally once.
  let wholeRallies = $state(false);
  const rallyOf = $derived(rallySpans(snap.groups));
  function windows(clips: Clip[]): Clip[] {
    return wholeRallies ? wholeRallyClips(clips, rallyOf) : clips;
  }
  function toggleWholeRallies() {
    wholeRallies = !wholeRallies;
    video.playlist = windows(result.clips);
  }
  /** Plays clip `i`'s whole rally once; the list carries on from there. */
  function playRally(i: number) {
    const c = result.clips[i];
    if (!c || !video.open) return;
    video.playlist = video.playlist.map((x, j) => (j === i ? asRally(c, rallyOf) : x));
    void video.playClip(i);
  }

  function modify(i: number) {
    const c = result.clips[i];
    if (!c) return;
    kept = i;
    checked[c.id] = true;
    ui.startEdit("correct", c.id, c.text);
  }

  // The filter folded to one line (v6 clips design); the form opens below.
  let filtering = $state(false);
  const summary = $derived(filterSummary(filter, snap));
  const checkedCount = $derived(result.clips.filter((c) => checked[c.id]).length);
  // Leaving the list stops it.
  onDestroy(() => video.stopClips());

  const fmt = (ms: number) => {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  };
</script>

<div class="clips" aria-label="Clips">
  <div class="clip-sum">
    <span class="txt" title={summary.join(" · ")}
      ><b>{summary[0]}</b> · {summary.slice(1).join(" · ")}</span
    >
    <button
      class="btn small"
      type="button"
      aria-expanded={filtering}
      aria-pressed={filtering}
      onclick={() => (filtering = !filtering)}>Filter</button
    >
  </div>
  {#if filtering}
    <CodeFilter {snap} bind:filter ondone={() => (filtering = false)} />
  {/if}
  <div class="clip-controls">
    {#if video.clipAt >= 0}
      <button class="btn small primary" type="button" onclick={video.stopClips}>Stop</button>
    {:else}
      <button
        class="btn small primary"
        type="button"
        disabled={!video.open || !result.clips.length}
        title={video.open ? "Play every clip in turn" : "Open the match video first"}
        onclick={() => video.playClip(0)}>Play all</button
      >
    {/if}
    <button
      class="btn small"
      type="button"
      disabled={!video.open || !result.clips.length}
      title="Previous clip ({keyFor(ui.keymap, 'clipPrev')})"
      onclick={() => video.stepClip(-1)}>Prev</button
    >
    <button
      class="btn small"
      type="button"
      disabled={!video.open || !result.clips.length}
      title="Next clip ({keyFor(ui.keymap, 'clipNext')})"
      onclick={() => video.stepClip(1)}>Next</button
    >
    <button
      class="btn small"
      type="button"
      aria-pressed={wholeRallies}
      title="Play each clip as its whole rally, serve to point (each rally once)"
      onclick={toggleWholeRallies}>Whole rallies</button
    >
    <span class="count"
      >{result.clips.length} clips · {checkedCount} checked{#if result.withoutVideo}
        · {result.withoutVideo} without video time{/if}</span
    >
  </div>
  <div class="tab-panel">
    {#if error}<div class="empty-note">{error}</div>{/if}
    {#each result.clips as c, i (c.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        role="row"
        tabindex="-1"
        class="row"
        class:picked={video.clipAt === i}
        class:home={c.team === "home"}
        class:away={c.team === "visiting"}
        title={video.open ? "Play from this clip" : undefined}
        onclick={() => video.open && video.playClip(i)}
      >
        <span class="stripe"></span>
        <span class="tc">S{c.set} {c.score.home}:{c.score.visiting}</span>
        <code>{c.text}</code>
        <span class="meaning">{c.meaning}</span>
        <span class="clip-tools">
          <span class="rate">{fmt(c.videoMs)}</span>
          <button
            type="button"
            class="btn small"
            disabled={!video.open || !rallyOf[c.id]}
            title="Play this clip's whole rally, serve to point"
            onclick={(e) => {
              e.stopPropagation();
              playRally(i);
            }}>Rally</button
          >
          <button
            type="button"
            class="btn small"
            title={withKey("Modify this code; the list keeps its place", "modifyPicked")}
            onclick={(e) => {
              e.stopPropagation();
              modify(i);
            }}>Modify</button
          >
          <label class="checked" title="Ticked: you've looked at this one"
            ><input
              type="checkbox"
              bind:checked={checked[c.id]}
              onclick={(e) => e.stopPropagation()}
            />Checked</label
          >
        </span>
      </div>
    {:else}
      <div class="empty-note">No clips match.</div>
    {/each}
  </div>
</div>

<style>
  .clips {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 8px 8px;
  }
  .clip-sum {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 4px 4px 8px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--sunk);
    flex: none;
  }
  .clip-sum .txt {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
  }
  .clip-sum b {
    font-weight: 600;
  }
  .clip-controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    flex: none;
  }
  .clip-controls .count {
    margin-left: auto;
    font-size: 12px;
    color: var(--muted);
  }
  .tab-panel {
    flex: 1;
    min-height: 0;
    padding: 0;
  }
  /* A clip row is two lines: the code and what it is, then its time and
     tools, so Rally, Modify and Checked never run off a narrow column. */
  .clips :global(.row) {
    /* In the scrolling list a row keeps its height (a flex item would
       shrink below its content, and the picked fill would stop short). */
    flex: none;
    grid-template-columns: 4px auto max-content minmax(0, 1fr);
    row-gap: 2px;
    padding-block: 3px;
  }
  .clips :global(.row .stripe) {
    grid-row: span 2;
  }
  .clip-tools {
    grid-column: 2 / -1;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .clip-tools .rate {
    margin-right: auto;
  }
  .checked {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 12px;
    color: var(--muted);
  }
</style>
