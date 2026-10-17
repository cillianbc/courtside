<script lang="ts">
  import * as backend from "../backend";
  import type { ClipFilter, Clips, Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";

  let { snap }: { snap: Snapshot } = $props();

  // The playlist the filter picks (PRD F-23), played clip by clip in the
  // video with the season's pre- and post-roll.
  let filter = $state<ClipFilter>({
    team: null,
    player: null,
    skill: null,
    evaluations: "",
    rotation: null,
    phase: null,
    set: null,
  });
  let result = $state<Clips>({ clips: [], withoutVideo: 0 });
  let error = $state("");

  $effect(() => {
    void snap;
    const f = $state.snapshot(filter);
    backend.clips(f).then(
      (c) => {
        result = c;
        error = "";
        // A new list: the clip playing is no longer where it was.
        video.stopClips();
        video.playlist = c.clips;
      },
      (e) => (error = String(e)),
    );
  });

  const SKILLS: [string, string][] = [
    ["S", "Serve"],
    ["R", "Reception"],
    ["E", "Set"],
    ["A", "Attack"],
    ["B", "Block"],
    ["D", "Dig"],
    ["F", "Free ball"],
  ];
  const EVALUATIONS = ["#", "+", "!", "-", "/", "="];

  const players = $derived(filter.team ? snap.teams[filter.team].players : []);

  function setTeam(team: Side | null) {
    filter.team = team;
    filter.player = null;
  }

  function toggleEvaluation(v: string) {
    filter.evaluations = filter.evaluations.includes(v)
      ? filter.evaluations.replace(v, "")
      : filter.evaluations + v;
  }

  const fmt = (ms: number) => {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  };
</script>

<section class="panel clips" aria-label="Clips">
  <div class="clip-filter">
    <label
      >Team
      <select
        value={filter.team ?? ""}
        onchange={(e) => setTeam((e.currentTarget.value || null) as Side | null)}
      >
        <option value="">Both</option>
        <option value="home">{snap.teams.home.name}</option>
        <option value="visiting">{snap.teams.visiting.name}</option>
      </select>
    </label>
    <label
      >Player
      <select bind:value={filter.player} disabled={!filter.team}>
        <option value={null}>All</option>
        {#each players as p (p.number)}<option value={p.number}>{p.number} {p.name}</option>{/each}
      </select>
    </label>
    <label
      >Skill
      <select bind:value={filter.skill}>
        <option value={null}>All</option>
        {#each SKILLS as [k, name] (k)}<option value={k}>{name}</option>{/each}
      </select>
    </label>
    <label
      >Rotation
      <select bind:value={filter.rotation} title="The touching team's setter's zone">
        <option value={null}>All</option>
        {#each [1, 2, 3, 4, 5, 6] as z (z)}<option value={z}>Setter in {z}</option>{/each}
      </select>
    </label>
    <label
      >Phase
      <select bind:value={filter.phase}>
        <option value={null}>All</option>
        <option value="serve">Serve</option>
        <option value="reception">Side-out</option>
        <option value="transition">Transition</option>
      </select>
    </label>
    <label
      >Set
      <select bind:value={filter.set}>
        <option value={null}>All</option>
        {#each snap.sets as s (s.number)}<option value={s.number}>{s.number}</option>{/each}
      </select>
    </label>
    <span class="evals" role="group" aria-label="Evaluations">
      {#each EVALUATIONS as v (v)}
        <button
          type="button"
          class="btn"
          aria-pressed={filter.evaluations.includes(v)}
          onclick={() => toggleEvaluation(v)}>{v}</button
        >
      {/each}
    </span>
  </div>
  <div class="clip-bar">
    <span class="rate"
      >{result.clips.length} clips{#if result.withoutVideo}
        · {result.withoutVideo} without video time (sync the video){/if}</span
    >
    <span class="stream-tools">
      {#if video.clipAt >= 0}
        <button class="btn" type="button" onclick={video.stopClips}>Stop</button>
      {:else}
        <button
          class="btn primary"
          type="button"
          disabled={!video.open || !result.clips.length}
          title={video.open ? "Play every clip in turn" : "Open the match video first"}
          onclick={() => video.playClip(0)}>Play all</button
        >
      {/if}
      <button
        class="btn"
        type="button"
        disabled={!video.open || !result.clips.length}
        title="Previous clip (Alt+[)"
        onclick={() => video.stepClip(-1)}>Prev</button
      >
      <button
        class="btn"
        type="button"
        disabled={!video.open || !result.clips.length}
        title="Next clip (Alt+])"
        onclick={() => video.stepClip(1)}>Next</button
      >
      <button
        class="btn"
        type="button"
        onclick={() => {
          video.stopClips();
          ui.clipsOpen = false;
        }}>Codes</button
      >
    </span>
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
        <span class="rate">{fmt(c.videoMs)}</span>
      </div>
    {:else}
      <div class="empty-note">No clips match.</div>
    {/each}
  </div>
</section>

<style>
  .clips {
    display: flex;
    flex-direction: column;
  }
  .clip-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    align-items: end;
    padding-bottom: 8px;
  }
  .clip-filter label {
    display: flex;
    flex-direction: column;
    font-size: 11px;
    gap: 2px;
  }
  .evals {
    display: flex;
    gap: 2px;
  }
  .evals .btn {
    min-width: 26px;
    padding-inline: 4px;
    font-family: var(--mono);
  }
  .evals .btn[aria-pressed="true"] {
    background: var(--accent);
    color: var(--on-accent);
  }
  .clip-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding-bottom: 6px;
  }
  .tab-panel {
    flex: 1;
  }
</style>
