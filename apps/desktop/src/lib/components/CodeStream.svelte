<script lang="ts">
  import type { Group, Row, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import CodeMenu from "./CodeMenu.svelte";

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

  // A lineup row is modified in the lineup form, which works on the set in
  // progress only.
  function canModify(r: Row, g: Group): boolean {
    if (r.kind === "goldenSet") return false;
    return r.kind !== "startingSix" || (g.set === snap.set && !snap.matchOver && r.team !== null);
  }

  function modify(r: Row) {
    if (r.kind === "startingSix") {
      ui.lineupFor = r.team;
      ui.subFor = null;
      ui.selected = null;
    } else ui.startEdit("correct", r.id, r.text);
  }

  let menu = $state<{ x: number; y: number; row: Row; group: Group } | null>(null);
</script>

<section class="panel codes" aria-label="Codes">
  <div class="set-tabs">
    <div class="set-list" role="tablist" aria-label="Sets">
      {#each snap.sets as s (s.number)}
        <button
          class="tab"
          role="tab"
          type="button"
          aria-label="Set {s.number}"
          aria-selected={s.number === shown}
          tabindex={s.number === shown ? 0 : -1}
          onclick={() => (ui.shownSet = s.number === snap.set ? null : s.number)}
        >
          {s.number === snap.goldenSet ? "GS" : `S${s.number}`}
          {#if s.finished}<span class="rate">{s.score.home}:{s.score.visiting}</span>{/if}
        </button>
      {/each}
    </div>
    <span class="stream-tools">
      <button
        class="btn"
        type="button"
        title="Undo the last code (Ctrl+Z)"
        onclick={() => ui.undo()}>Undo</button
      >
      <button class="btn" type="button" title="Redo (Ctrl+Shift+Z)" onclick={() => ui.redo()}
        >Redo</button
      >
    </span>
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
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            role="row"
            tabindex="-1"
            onclick={() => (ui.selected = ui.selected === r.id ? null : r.id)}
            oncontextmenu={(e) => {
              e.preventDefault();
              ui.selected = r.id;
              menu = { x: e.clientX, y: e.clientY, row: r, group: g };
            }}
            class:picked={ui.selected === r.id}
            class:edited={r.edited}
            class="row"
            class:home={r.team === "home"}
            class:away={r.team === "visiting"}
            class:sys={r.team === null}
            class:auto={r.auto}
            title={r.auto ? "Written by the console" : undefined}
            id={`ev-${r.id}`}
          >
            <span class="stripe"></span>
            <span class="tc">{r.index + 1}</span>
            <code>{r.text}</code>
            <span class="meaning">{r.meaning}</span>
            {#if ui.selected === r.id}
              <span class="row-actions">
                {#if canModify(r, g)}
                  <button
                    type="button"
                    class="btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      modify(r);
                    }}>Modify</button
                  >
                {/if}
                <button
                  type="button"
                  class="btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    ui.startEdit("insert", r.id, r.text);
                  }}>Add before</button
                >
                <button
                  type="button"
                  class="btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    ui.startEdit("after", r.id, r.text);
                  }}>Add after</button
                >
                <button
                  type="button"
                  class="btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    void ui.remove(r.id, r.text);
                  }}>Remove</button
                >
              </span>
            {:else if r.severity}
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
  {#if menu}
    {@const m = menu}
    <CodeMenu
      x={m.x}
      y={m.y}
      onclose={() => (menu = null)}
      items={[
        { label: "Modify", disabled: !canModify(m.row, m.group), run: () => modify(m.row) },
        { label: "Add before", run: () => ui.startEdit("insert", m.row.id, m.row.text) },
        { label: "Add after", run: () => ui.startEdit("after", m.row.id, m.row.text) },
        { label: "Remove", danger: true, run: () => void ui.remove(m.row.id, m.row.text) },
      ]}
    />
  {/if}
</section>
