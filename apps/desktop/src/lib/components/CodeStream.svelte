<script lang="ts">
  import { stepFound } from "../actions";
  import * as backend from "../backend";
  import { filterSummary, hasFields } from "../filter";
  import type { Group, IssueView, Row, Snapshot } from "../snapshot";
  import { showCode } from "../jump";
  import { withKey } from "../kbd";
  import { ui } from "../ui.svelte";
  import { video } from "../video.svelte";
  import CodeFilter from "./CodeFilter.svelte";
  import CodeMenu from "./CodeMenu.svelte";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";

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

  // The score before a rally: the one before it in the set, else 0:0.
  function scoreBefore(i: number): string {
    for (let j = i - 1; j >= 0; j--) {
      const s = groups[j]?.scoreAfter;
      if (s) return `${s.home}:${s.visiting}`;
    }
    return "0:0";
  }

  // Picking a code shows it in the video, where it has a video time.
  function pick(r: Row) {
    ui.selected = ui.selected === r.id ? null : r.id;
    if (ui.selected && r.videoMs !== null && video.open) void video.seekTo(r.videoMs / 1000);
  }

  // A code whose issues all follow from an earlier fault: that fault.
  const causeOf = $derived.by(() => {
    const own = new Set(snap.issues.filter((i) => !i.causeId).map((i) => i.eventId));
    const cause: Record<string, string> = {};
    for (const i of snap.issues) {
      if (i.causeId && !own.has(i.eventId)) cause[i.eventId] ??= i.causeId;
    }
    return cause;
  });
  // Match-wide code numbers by event id, once per snapshot (v6 T4).
  const numberOf = $derived.by(() => {
    const n: Record<string, number> = {};
    for (const g of snap.groups) for (const r of g.rows) n[r.id] = r.index + 1;
    return n;
  });
  // The first issue on each code, for its mark's tooltip.
  const issueOf = $derived.by(() => {
    const m: Record<string, IssueView> = {};
    for (const i of snap.issues) m[i.eventId] ??= i;
    return m;
  });
  // A long gap before a serve (the missed-rally check) shows between the
  // rally heads it falls between, by the code it was found on.
  const gapBefore = $derived.by(() => {
    const m: Record<string, IssueView> = {};
    for (const i of snap.issues) {
      if (i.severity === "info" && i.message.includes("since the last point")) m[i.eventId] = i;
    }
    return m;
  });
  function gapIn(g: Group): IssueView | undefined {
    for (const r of g.rows) if (gapBefore[r.id]) return gapBefore[r.id];
    return undefined;
  }
  // Root faults and the issues that follow from them, for the log's badge.
  const faults = $derived({
    roots: snap.issues.filter((i) => i.severity === "error" && !i.knockOn).length,
    follow: snap.issues.filter((i) => i.knockOn).length,
  });

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

  const picked = $derived.by(() => {
    for (const g of groups) {
      const r = g.rows.find((x) => x.id === ui.selected);
      if (r) return { row: r, group: g };
    }
    return null;
  });

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
  const foundSet = $derived(new Set(ui.found));
  const asked = $derived(!!ui.findFilter.text || hasFields(ui.findFilter));
  const foundAt = $derived(ui.selected ? ui.found.indexOf(ui.selected) : -1);
  function hits(g: Group): number {
    return ui.finding ? g.rows.filter((r) => foundSet.has(r.id)).length : 0;
  }
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

  let section: HTMLElement | undefined = $state();
  let menu = $state<{ x: number; y: number; row: Row; group: Group } | null>(null);
</script>

<section class="panel codes" aria-label="Codes" bind:this={section}>
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
    <button
      class="btn lineup-small"
      type="button"
      title="Both teams' starting sixes for the set shown"
      onclick={() => (ui.lineupScreen = shown)}><Icon name="lineup" />Lineup</button
    >
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
  <div class="log-head">
    <span class="panel-title">Codes</span>
    {#if faults.roots}
      <button
        class="flag err"
        type="button"
        title={withKey("Open Checks here; the video stays as it is", "openChecks")}
        onclick={() => ui.openChecks()}
        ><Mark severity="error" />{faults.roots}
        {faults.roots === 1 ? "fault" : "faults"}{#if faults.follow}&nbsp;· {faults.follow} follow{/if}</button
      >
    {/if}
    <span class="stream-tools">
      <button
        class="btn icon"
        type="button"
        aria-label="Search the codes"
        aria-pressed={ui.finding}
        title={withKey("Search the codes and step through what it finds", "find")}
        onclick={() => (ui.finding ? ui.closeFind() : ui.openFind())}><Icon name="find" /></button
      >
      <button
        class="btn"
        type="button"
        title="Play the codes as video clips, picked by player, skill, rotation or phase"
        onclick={() => ui.openClips()}><Icon name="clips" />Clips</button
      >
      <button
        class="btn icon"
        type="button"
        aria-label="Undo"
        title={withKey("Undo the last code", "undo")}
        onclick={() => ui.undo()}><Icon name="undo" /></button
      >
      <button class="btn" type="button" title={withKey("Redo", "redo")} onclick={() => ui.redo()}
        >Redo</button
      >
    </span>
  </div>
  {#if ui.finding}
    <div class="find-bar" role="search" aria-label="Search the codes">
      <input
        bind:this={findInput}
        bind:value={ui.findFilter.text}
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder="Find: D=, a12, *T"
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
  {/if}
  <div class="tab-panel" role="tabpanel" bind:this={pane}>
    <div class="log" role="listbox" aria-label="Codes, set {shown}">
      {#each groups as g, i (key(g))}
        {@const closed = folded(g, i)}
        {@const first = firstTouch(g)}
        {@const gap = gapIn(g)}
        {#if gap}
          <div class="gap-hint">
            <Mark severity="info" />
            <span>{gap.message}</span>
            {#if first}
              <button
                class="rally-add"
                type="button"
                onclick={() => ui.startEdit("rally", first.id, first.text)}>+ Rally</button
              >
            {/if}
          </div>
        {/if}
        <div class="rally" class:closed role="group" aria-label="Rally {g.rally ?? ''}">
          <div class="rally-head">
            {#if g.closed}
              <button
                class="rally-toggle"
                type="button"
                aria-expanded={!closed}
                onclick={() => toggle(g)}
              >
                <span class="chev" class:open={!closed}>›</span>
                <b>Rally {g.rally}</b>
                <span class="num"
                  >{scoreBefore(i)} → {g.scoreAfter
                    ? `${g.scoreAfter.home}:${g.scoreAfter.visiting}`
                    : ""}</span
                >
                <span class="out {g.wonBy === 'home' ? 'home' : 'away'}">{teamName(g.wonBy)}</span>
                {#if hits(g)}<span class="hits" title="Codes the search found here">{hits(g)}</span
                  >{/if}
              </button>
            {:else}
              <span class="rally-toggle">
                <b>Rally {g.rally ?? ""}</b>
                <span class="num">{scoreBefore(i)}</span>
                <span class="out live"
                  >{g.rows.length
                    ? "in progress"
                    : g.serving
                      ? `${teamName(g.serving)} to serve`
                      : ""}</span
                >
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
              role="option"
              tabindex="-1"
              aria-selected={ui.selected === r.id}
              onclick={() => pick(r)}
              oncontextmenu={(e) => {
                e.preventDefault();
                ui.selected = r.id;
                menu = { x: e.clientX, y: e.clientY, row: r, group: g };
              }}
              class:picked={ui.selected === r.id}
              class:found={ui.finding && foundSet.has(r.id)}
              class="row"
              class:home={r.team === "home"}
              class:away={r.team === "visiting"}
              class:sys={r.team === null}
              class:auto={r.auto}
              title={r.meaning}
              id={`ev-${r.id}`}
            >
              <span class="stripe"></span>
              <span class="tc">{numberOf[r.id]}</span>
              <code>{r.text}</code>
              <span class="meaning"
                >{#if r.auto}<span class="tag auto" title="Written by the console">added</span
                  >{/if}{#if r.edited}<span class="tag edited">edited</span>{/if}{r.meaning}</span
              >
              <span class="flags">
                {#if causeOf[r.id]}
                  {@const cause = causeOf[r.id] ?? ""}
                  {@const n = numberOf[cause]}
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
                  <span title={issueOf[r.id]?.message ?? r.severity}
                    ><Mark severity={r.severity} label={issueOf[r.id]?.message} /></span
                  >
                {/if}
              </span>
            </div>
          {:else}
            <div class="empty-note">No codes yet. Start with the serve.</div>
          {/each}
        </div>
      {/each}
    </div>
  </div>
  {#if picked}
    {@const p = picked}
    <div class="pick-bar" role="toolbar" aria-label="Code {numberOf[p.row.id]}">
      <span class="picked-label">Code {numberOf[p.row.id]}</span>
      {#if canModify(p.row) && ui.editing?.id !== p.row.id}
        <button
          type="button"
          class="btn"
          title={withKey("Modify this code", "modifyPicked")}
          onclick={() => modify(p.row, p.group)}>Modify</button
        >
      {/if}
      <button
        type="button"
        class="btn"
        title={withKey("Add codes before this one", "insertBeforePicked")}
        onclick={() => ui.startEdit("insert", p.row.id, p.row.text)}>+ Before</button
      >
      <button
        type="button"
        class="btn"
        title={withKey("Add codes after this one", "insertAfterPicked")}
        onclick={() => ui.startEdit("after", p.row.id, p.row.text)}>+ After</button
      >
      <button type="button" class="btn" onclick={() => (ui.selected = null)}>Done</button>
      <button
        type="button"
        class="btn danger"
        title={withKey("Remove this code (undo brings it back)", "removePicked")}
        onclick={() => void ui.remove(p.row.id, p.row.text)}>Remove</button
      >
    </div>
  {/if}
  {#if menu}
    {@const m = menu}
    <CodeMenu
      x={m.x}
      y={m.y}
      within={section}
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
