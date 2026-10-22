<!-- Starting lineups for a set, both teams side by side: its own screen
     while a team still needs its six, and from the lineup button to
     modify a set's sixes. Zones are laid out as each team faces the net. -->
<script lang="ts">
  import SetterBadge from "./SetterBadge.svelte";
  import { untrack } from "svelte";
  import * as backend from "../backend";
  import { startOf, type Side, type Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import MatchRosterDialog from "./MatchRosterDialog.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap, set }: { snap: Snapshot; set: number } = $props();

  // Front row at the net, then the back row, as the team sees it.
  const GRID = [4, 3, 2, 5, 6, 1] as const;

  interface Draft {
    /** Shirt numbers by zone (index zone − 1), as typed. */
    zones: string[];
    setter: string;
  }

  // Filled once from the set's sixes; later snapshots don't overwrite typing.
  const draft = $state<Record<Side, Draft>>(
    untrack(() => ({ home: fromStart("home"), visiting: fromStart("visiting") })),
  );
  let note = $state("");
  let rosterFor = $state<Side | null>(null);
  let busy = $state(false);

  function fromStart(side: Side): Draft {
    const start = startOf(snap, set, side);
    return {
      zones: Array.from({ length: 6 }, (_, i) => String(start?.court[i] ?? "")),
      setter: start?.setter != null ? String(start.setter) : "",
    };
  }

  const needs = $derived(snap.needsLineup.home || snap.needsLineup.visiting);

  function court(side: Side): number[] {
    return draft[side].zones
      .map((z) => z.trim())
      .filter((z) => z !== "")
      .map(Number)
      .filter((n) => isShirt(n));
  }
  function isShirt(n: number): boolean {
    return Number.isInteger(n) && n >= 0 && n <= 99;
  }

  // The setter by role when one is set, else the one typed or picked.
  function setterOf(side: Side, six: number[]): number | null {
    const typed = draft[side].setter.trim();
    if (typed) return Number(typed);
    const byRole = snap.teams[side].players.filter((p) => p.role === 5 && six.includes(p.number));
    return byRole.length === 1 ? (byRole[0]?.number ?? null) : null;
  }

  /** What stops a team's six from going in, if anything. */
  function problem(side: Side): string | null {
    const filled = draft[side].zones.filter((z) => z.trim() !== "");
    if (filled.length < 6)
      return `${6 - filled.length} zone${filled.length === 5 ? "" : "s"} empty`;
    const six = court(side);
    if (six.length < 6) return "a zone isn't a shirt number";
    if (new Set(six).size < 6) return "a player is in two zones";
    const sheet = snap.teams[side].players.map((p) => p.number);
    const missing = six.find((n) => !sheet.includes(n));
    if (missing !== undefined) return `#${missing} isn't on the team sheet`;
    const s = setterOf(side, six);
    if (s === null || !six.includes(s)) return "pick the setter";
    return null;
  }

  function changed(side: Side): boolean {
    const start = fromStart(side);
    return (
      start.zones.join(" ") !== draft[side].zones.map((z) => z.trim()).join(" ") ||
      start.setter !== draft[side].setter.trim()
    );
  }

  const problems = $derived(ui.order.map((side) => [side, problem(side)] as const));
  const ready = $derived(problems.every(([, p]) => p === null));

  // A team-sheet chip fills the first empty zone, or takes its player out.
  function toggle(side: Side, n: number) {
    const zones = draft[side].zones;
    const at = zones.findIndex((z) => z.trim() === String(n));
    if (at >= 0) zones[at] = "";
    else {
      const empty = zones.findIndex((z) => z.trim() === "");
      if (empty < 0) {
        note = "All six zones are filled: clear one first.";
        return;
      }
      zones[empty] = String(n);
    }
    note = "";
  }

  async function save() {
    if (!ready || busy) return;
    busy = true;
    note = "";
    for (const side of ui.order) {
      const six = court(side);
      const s = setterOf(side, six);
      if (s === null) continue;
      const start = startOf(snap, set, side);
      const name = snap.teams[side].name;
      if (start && !changed(side)) continue;
      const ok = start
        ? await ui.apply(`${name} lineup modified`, () => backend.correctLineup(side, set, six, s))
        : await ui.apply(`${name} lineup entered`, () => backend.setLineup(side, six, s));
      if (!ok) {
        note = ui.result?.text ?? `${name}'s lineup wasn't saved.`;
        busy = false;
        return;
      }
    }
    busy = false;
    ui.lineupScreen = null;
  }
</script>

<section class="lineup-screen" aria-label="Starting lineup">
  <h1>
    Starting lineup · {set === snap.goldenSet ? "golden set" : `set ${set}`}
    <small
      >Type each zone's shirt number, or click players on the team sheet to fill the next empty
      zone. Zones are as each team faces the net.</small
    >
  </h1>
  <div class="lineup-teams">
    {#each ui.order as side (side)}
      {@const cls = side === "home" ? "home" : "away"}
      {@const team = snap.teams[side]}
      {@const six = court(side)}
      <div class="lu-team {cls}">
        <div class="lu-head">
          <span class="team-name {cls}"><SideTag {side} />{team.name}</span>
          <button class="link" type="button" onclick={() => (rosterFor = side)}
            >Edit team sheet</button
          >
        </div>
        <div class="lu-court">
          <div class="net-cap">net</div>
          {#each GRID as z (z)}
            {@const n = Number(draft[side].zones[z - 1])}
            <div class="lu-zone" class:setter={n > 0 && n === setterOf(side, six)}>
              <label for="lu-{cls}-{z}">zone {z}</label>
              <input
                id="lu-{cls}-{z}"
                inputmode="numeric"
                maxlength="2"
                autocomplete="off"
                bind:value={draft[side].zones[z - 1]}
              />
              {#if n > 0 && n === setterOf(side, six)}<span class="lu-setter"
                  ><SetterBadge />setter</span
                >{/if}
            </div>
          {/each}
        </div>
        <div class="lu-row">
          <label for="lu-setter-{cls}">Setter</label>
          <select id="lu-setter-{cls}" bind:value={draft[side].setter}>
            <option value="">by role</option>
            {#each [...new Set(six)] as n (n)}<option value={String(n)}>#{n}</option>{/each}
          </select>
          <span class="sub-note" class:bad={problems.find(([s]) => s === side)?.[1] !== null}
            >{problems.find(([s]) => s === side)?.[1] ?? "ready"}</span
          >
        </div>
        <div>
          <div class="sub-note lu-sheet-note">Team sheet: click to fill the next empty zone</div>
          <div class="lu-bench">
            {#each [...team.players].sort((a, b) => a.number - b.number) as p (p.number)}
              <button
                type="button"
                class:used={six.includes(p.number)}
                class:lib={p.libero}
                disabled={p.libero}
                title={p.libero ? `${p.name}, libero: not in the six` : p.name}
                onclick={() => toggle(side, p.number)}
                >{p.number}{#if p.libero}&nbsp;L{/if}</button
              >
            {/each}
          </div>
        </div>
      </div>
    {/each}
  </div>
  <div class="lu-actions">
    <span class="sub-note" class:bad={note !== ""}
      >{note ||
        (ready
          ? "Both sixes are in."
          : needs
            ? "Enter both sixes to start the set."
            : "Fix the sixes above to save them.")}</span
    >
    {#if !needs}
      <button class="btn" type="button" onclick={() => (ui.lineupScreen = null)}
        >Back to the match</button
      >
    {/if}
    <button class="btn primary" type="button" disabled={!ready || busy} onclick={save}
      >{needs
        ? `Start ${set === snap.goldenSet ? "the golden set" : `set ${set}`}`
        : "Save lineups"}</button
    >
  </div>
</section>

{#if rosterFor}
  <MatchRosterDialog
    side={rosterFor}
    name={snap.teams[rosterFor].name}
    onclose={() => (rosterFor = null)}
  />
{/if}
