<script lang="ts">
  import { prefix, type Side, type Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import MatchRosterDialog from "./MatchRosterDialog.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap }: { snap: Snapshot } = $props();
  let editing = $state<Side | null>(null);

  function players(side: Side) {
    const court = snap.court[side];
    const all = [...snap.teams[side].players].sort((a, b) => a.number - b.number);
    const on = all.filter((p) => court.includes(p.number));
    const off = all.filter((p) => !court.includes(p.number));
    return [...on, ...off].map((p) => {
      const zone = court.indexOf(p.number);
      const mark = p.libero
        ? "L"
        : p.number === snap.setter[side]
          ? "S"
          : zone >= 0
            ? `Z${zone + 1}`
            : "";
      return { ...p, on: zone >= 0, mark };
    });
  }
</script>

<section class="panel" aria-label="Rosters">
  <h2>
    Rosters <span style="text-transform:none;letter-spacing:0;font-weight:400"
      >click a player to start a code</span
    >
  </h2>
  <div class="rosters">
    {#each ui.order as side (side)}
      {@const cls = side === "home" ? "home" : "away"}
      <div class="roster">
        <h3 class={cls}>
          <SideTag {side} />{snap.teams[side].name}
          <button
            class="link edit-roster"
            type="button"
            title="Edit this match's roster"
            onclick={() => (editing = side)}>edit</button
          >
        </h3>
        {#each players(side) as p (p.number)}
          <button
            type="button"
            class="pl {cls}"
            class:on={p.on}
            class:bench={!p.on}
            title={`Start a code for ${snap.teams[side].name} #${p.number}`}
            onclick={() => ui.startCode(prefix(side) + String(p.number).padStart(2, "0"))}
          >
            <span class="no">{p.number}</span><span class="nm">{p.name}</span><span class="mk"
              >{p.mark}</span
            >
          </button>
        {/each}
      </div>
    {/each}
  </div>
</section>
{#if editing}
  <MatchRosterDialog
    side={editing}
    name={snap.teams[editing].name}
    onclose={() => (editing = null)}
  />
{/if}
