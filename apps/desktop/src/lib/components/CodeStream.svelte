<script lang="ts">
  import type { Group, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const shown = $derived(ui.shownSet ?? snap.set);
  const groups = $derived(snap.groups.filter((g) => g.set === shown));
  const last = $derived(groups.length - 1);
  // Older closed rallies start folded; clicking a head toggles one.
  let toggled = $state<Record<string, boolean>>({});

  let pane: HTMLDivElement | undefined = $state();
  $effect(() => {
    void groups;
    if (pane && ui.shownSet === null) pane.scrollTop = pane.scrollHeight;
  });

  function key(g: Group): string {
    return g.rows[0]?.id ?? `open-${g.set}`;
  }

  function folded(g: Group, i: number): boolean {
    const byDefault = g.closed && i < last - 2;
    return toggled[key(g)] ? !byDefault : byDefault;
  }

  function toggle(g: Group) {
    const k = key(g);
    toggled[k] = !toggled[k];
  }

  function teamName(side: "home" | "visiting" | null): string {
    return side ? snap.teams[side].name : "";
  }

  const FLAG = { error: "err", warning: "warn", info: "info" } as const;
</script>

<section class="panel codes" aria-label="Codes">
  <div class="set-tabs" role="tablist" aria-label="Sets">
    {#each snap.sets as s (s.number)}
      <button
        class="tab"
        role="tab"
        type="button"
        aria-selected={s.number === shown}
        tabindex={s.number === shown ? 0 : -1}
        onclick={() => (ui.shownSet = s.number === snap.set ? null : s.number)}
      >
        Set {s.number}
        {#if s.finished}<span class="rate">{s.score.home}:{s.score.visiting}</span>{/if}
      </button>
    {/each}
  </div>
  <div class="tab-panel" role="tabpanel" bind:this={pane}>
    {#each groups as g, i (key(g))}
      {@const closed = folded(g, i)}
      <div class="rally" class:closed>
        {#if g.closed}
          <div
            class="rally-head"
            role="button"
            tabindex="0"
            aria-expanded={!closed}
            onclick={() => toggle(g)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                toggle(g);
              }
            }}
          >
            <span>Rally {g.rally} · {g.rows.length} codes</span>
            <span
              >{teamName(g.wonBy)}
              {#if g.scoreAfter}<b>{g.scoreAfter.home}:{g.scoreAfter.visiting}</b>{/if}</span
            >
          </div>
        {:else}
          <div class="rally-head">
            <span>Rally {g.rally ?? ""} · {g.rows.length ? "in progress" : "next"}</span>
            <span>{g.serving ? `${teamName(g.serving)} serving` : ""}</span>
          </div>
        {/if}
        {#each g.rows as r (r.id)}
          <div
            class="row"
            class:home={r.team === "home"}
            class:away={r.team === "visiting"}
            class:sys={r.team === null}
            id={`ev-${r.id}`}
          >
            <span class="stripe"></span>
            <span class="tc">{r.index + 1}</span>
            <code>{r.text}</code>
            <span class="meaning"></span>
            {#if r.severity}
              <span class="flag {FLAG[r.severity]}">{r.severity}</span>
            {:else}
              <span></span>
            {/if}
          </div>
        {:else}
          <div class="empty-note">No codes yet. Start with the serve.</div>
        {/each}
      </div>
    {/each}
  </div>
</section>
