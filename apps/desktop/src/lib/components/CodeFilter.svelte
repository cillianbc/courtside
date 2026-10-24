<script lang="ts">
  import { emptyFilter, SKILLS } from "../filter";
  import type { ClipFilter, Side, Snapshot } from "../snapshot";

  // Which codes, for Clips and for the code log's search (#25). `withText`:
  // a field for the code text (the search bar has its own).
  let {
    snap,
    filter = $bindable(),
    withText = true,
    ondone,
  }: { snap: Snapshot; filter: ClipFilter; withText?: boolean; ondone: () => void } = $props();

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

  function clear() {
    filter = { ...emptyFilter(), text: withText ? "" : filter.text };
  }
</script>

<div class="code-filter">
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
    >Set
    <select bind:value={filter.set}>
      <option value={null}>All</option>
      {#each snap.sets as s (s.number)}<option value={s.number}>{s.number}</option>{/each}
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
  {#if withText}
    <label class="text"
      >Code text
      <input
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="D= or a12"
        bind:value={filter.text}
      />
    </label>
  {/if}
  <span class="evals" role="group" aria-label="Evaluations">
    {#each EVALUATIONS as v (v)}
      <button
        type="button"
        class="btn small"
        aria-pressed={filter.evaluations.includes(v)}
        onclick={() => toggleEvaluation(v)}>{v}</button
      >
    {/each}
  </span>
  <span class="filter-done">
    <button class="btn small" type="button" onclick={clear}>Clear</button>
    <button class="btn small primary" type="button" onclick={ondone}>Done</button>
  </span>
</div>

<style>
  .code-filter {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px;
    padding: 8px;
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
    background: var(--panel);
    flex: none;
  }
  .code-filter label {
    display: grid;
    gap: 2px;
    font-size: 12px;
    color: var(--muted);
    min-width: 0;
  }
  /* A select is as wide as its longest option (a team's name) unless told
     otherwise: it fills its column instead and never spills into the next. */
  .code-filter select,
  .code-filter input {
    width: 100%;
    min-width: 0;
    text-overflow: ellipsis;
    font: inherit;
    font-size: 13px;
    height: 28px;
    color: var(--fg);
    background: var(--sunk);
    border: 1px solid var(--edge);
    border-radius: var(--radius-control);
  }
  .code-filter input {
    font-family: var(--mono);
    padding: 0 6px;
  }
  .text {
    grid-column: 1 / -1;
  }
  .evals {
    grid-column: 1 / -1;
    display: flex;
    gap: 3px;
  }
  .evals .btn {
    min-width: 28px;
    font-family: var(--mono);
  }
  .filter-done {
    grid-column: 1 / -1;
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
</style>
