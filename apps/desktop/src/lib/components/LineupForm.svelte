<script lang="ts">
  import { untrack } from "svelte";
  import * as backend from "../backend";
  import { parseSix } from "../codes";
  import { startOf, type Side, type Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import SideTag from "./SideTag.svelte";

  // `onclose` set: modifying set `set`'s lineup, starting from the six it
  // started with; otherwise entering the lineup for a new set.
  let {
    snap,
    side,
    set,
    onclose,
  }: { snap: Snapshot; side: Side; set?: number; onclose?: () => void } = $props();

  // Filled once when the form opens; later snapshots don't overwrite typing.
  const forSet = $derived(set ?? snap.set);
  const start = untrack(() => (onclose ? startOf(snap, set ?? snap.set, side) : null));
  let six = $state(start ? start.court.map((n) => n ?? "").join(" ") : "");
  let setter = $state(start?.setter != null ? String(start.setter) : "");
  let note = $state("");

  const team = $derived(snap.teams[side]);
  const players = $derived(
    team.players
      .filter((p) => !p.libero)
      .sort((a, b) => a.number - b.number)
      .map((p) => (p.role === 5 ? `${p.number}(S)` : String(p.number))),
  );

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    const read = parseSix(six);
    if ("error" in read) {
      note = read.error;
      return;
    }
    // Left empty: the one player in the six whose role is setter.
    const setters = team.players.filter((p) => p.role === 5 && read.court.includes(p.number));
    const s = setter.trim()
      ? Number(setter)
      : setters.length === 1
        ? (setters[0]?.number ?? -1)
        : -1;
    if (!read.court.includes(s)) {
      note = "The setter must be one of the six.";
      return;
    }
    note = "";
    const ok = onclose
      ? await ui.apply(`${team.name} lineup modified`, () =>
          backend.correctLineup(side, forSet, read.court, s),
        )
      : await ui.apply(`${team.name} lineup entered`, () => backend.setLineup(side, read.court, s));
    if (ok && onclose) onclose();
    else if (ok) {
      six = "";
      setter = "";
    } else {
      // Said here, not only in the command bar far below.
      const why = ui.result?.text ?? "The lineup wasn't entered.";
      note = why.includes("not on the team sheet")
        ? `${why}: add the player to this match's roster first: "edit" by the team's name in the rosters.`
        : why;
    }
  }
</script>

<form class="sub-form" onsubmit={submit}>
  <div class="title">
    <SideTag {side} />
    {team.name}: {onclose ? "modify the lineup" : "lineup"} for set {forSet}
  </div>
  <div class="pick">
    <label
      >Zones 1–6
      <input class="lineup-six" bind:value={six} placeholder="8 12 13 11 3 14" autocomplete="off" />
    </label>
    <label
      >Setter
      <input class="lineup-setter" bind:value={setter} placeholder="auto" autocomplete="off" />
    </label>
    <button class="btn primary" type="submit">{onclose ? "Modify lineup" : "Enter lineup"}</button>
    {#if onclose}<button class="btn" type="button" onclick={onclose}>Cancel</button>{/if}
  </div>
  <div class="sub-note" class:bad={note !== ""}>
    {note ||
      `Players: ${players.join(" ")}. Setter left empty: the six's setter by role.` +
        (onclose ? " The set replays from the modified six." : "")}
  </div>
</form>
