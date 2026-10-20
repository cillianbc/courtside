<script lang="ts">
  import type { Group, Row, Snapshot } from "../snapshot";
  import { showCode } from "../jump";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";
  import CodeMenu from "./CodeMenu.svelte";
  import Icon from "./Icon.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const shown = $derived(ui.shownSet ?? snap.set);
  const groups = $derived(snap.groups.filter((g) => g.set === shown));
  const last = $derived(groups.length - 1);
  // Older closed rallies start folded; clicking a head toggles one.
  let toggled = $state<Record<string, boolean>>({});

  let pane: HTMLDivElement | undefined = $state();
  $effect(() => {
    void groups;
    // An edit of an earlier code keeps the log where the scout is working.
    if (ui.keepPlace) {
      ui.keepPlace = false;
      return;
    }
    if (pane && ui.shownSet === null) pane.scrollTop = pane.scrollHeight;
  });

  // By set and rally, so modifying a rally's first code keeps it open.
  function key(g: Group): string {
    return g.rally === null ? (g.rows[0]?.id ?? `open-${g.set}`) : `${g.set}-${g.rally}`;
  }

  function folded(g: Group, i: number): boolean {
    const byDefault = g.closed && i < last - 2;
    return toggled[key(g)] ? !byDefault : byDefault;
  }

  // A missed rally goes in before this rally's first touch: after the
  // rotation that led to it.
  function firstTouch(g: Group): Row | undefined {
    return g.rows.find((r) => r.kind === "touch");
  }

  function toggle(g: Group) {
    const k = key(g);
    toggled[k] = !toggled[k];
  }

  function teamName(side: "home" | "visiting" | null): string {
    return side ? snap.teams[side].name : "";
  }

  // Picking a code shows it in the video, where it has a video time.
  function pick(r: Row) {
    ui.selected = ui.selected === r.id ? null : r.id;
    if (ui.selected && r.videoMs !== null && video.open) void video.seekTo(r.videoMs / 1000);
  }

  const FLAG = { error: "err", warning: "warn", info: "info" } as const;
  // A code whose issues all follow from an earlier fault: that fault.
  const causeOf = $derived.by(() => {
    const own = new Set(snap.issues.filter((i) => !i.causeId).map((i) => i.eventId));
    const cause: Record<string, string> = {};
    for (const i of snap.issues) {
      if (i.causeId && !own.has(i.eventId)) cause[i.eventId] ??= i.causeId;
    }
    return cause;
  });
  function faultNumber(id: string): number | null {
    for (const g of snap.groups) {
      const r = g.rows.find((x) => x.id === id);
      if (r) return r.index + 1;
    }
    return null;
  }

  // A lineup row is modified in the lineup form, for the row's set.
  function canModify(r: Row): boolean {
    if (r.kind === "goldenSet") return false;
    return r.kind !== "startingSix" || r.team !== null;
  }

  function modify(r: Row, g: Group) {
    if (r.kind === "startingSix") {
      ui.lineupScreen = g.set;
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
        title="Play the codes as video clips, picked by player, skill, rotation or phase"
        onclick={() => (ui.clipsOpen = true)}>Clips</button
      >
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
  <button
    class="btn lineup-btn"
    type="button"
    title="Both teams' starting sixes for the set shown"
    onclick={() => (ui.lineupScreen = shown)}
    ><Icon name="lineup" />Starting lineup · {shown === snap.goldenSet
      ? "golden set"
      : `set ${shown}`}</button
  >
  <div class="tab-panel" role="tabpanel" bind:this={pane}>
    {#each groups as g, i (key(g))}
      {@const closed = folded(g, i)}
      {@const first = firstTouch(g)}
      <div class="rally" class:closed>
        <div class="rally-head">
          {#if g.closed}
            <button
              class="rally-toggle"
              type="button"
              aria-expanded={!closed}
              onclick={() => toggle(g)}
            >
              <span>Rally {g.rally} · {g.rows.length} codes</span>
              <span
                >{teamName(g.wonBy)}
                {#if g.scoreAfter}<b>{g.scoreAfter.home}:{g.scoreAfter.visiting}</b>{/if}</span
              >
            </button>
          {:else}
            <span class="rally-toggle">
              <span>Rally {g.rally ?? ""} · {g.rows.length ? "in progress" : "next"}</span>
              <span>{g.serving ? `${teamName(g.serving)} serving` : ""}</span>
            </span>
          {/if}
          {#if first}
            <button
              class="rally-add"
              type="button"
              title="Add a missed rally before this one: type its codes, Enter. The point is written and later scores move on."
              onclick={() => ui.startEdit("rally", first.id, first.text)}>+ Rally</button
            >
          {/if}
        </div>
        {#each g.rows as r (r.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            role="row"
            tabindex="-1"
            onclick={() => pick(r)}
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
                {#if canModify(r)}
                  <button
                    type="button"
                    class="btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      modify(r, g);
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
            {:else if causeOf[r.id]}
              {@const cause = causeOf[r.id] ?? ""}
              {@const n = faultNumber(cause)}
              <button
                type="button"
                class="follows"
                title="This follows from the fault at code {n ??
                  '?'}: fix that one first. Click to go there."
                onclick={(e) => {
                  e.stopPropagation();
                  showCode(snap, cause);
                }}>↳ {n ?? "fault"}</button
              >
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
        { label: "Modify", disabled: !canModify(m.row), run: () => modify(m.row, m.group) },
        { label: "Add before", run: () => ui.startEdit("insert", m.row.id, m.row.text) },
        { label: "Add after", run: () => ui.startEdit("after", m.row.id, m.row.text) },
        { label: "Remove", danger: true, run: () => void ui.remove(m.row.id, m.row.text) },
      ]}
    />
  {/if}
</section>
