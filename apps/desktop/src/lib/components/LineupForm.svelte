<script lang="ts">
  import * as backend from "../backend";
  import { parseSix } from "../codes";
  import type { Side, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap, side }: { snap: Snapshot; side: Side } = $props();

  let six = $state("");
  let setter = $state("");
  let note = $state("");

  const team = $derived(snap.teams[side]);
  const players = $derived(
    team.players
      .filter((p) => !p.libero)
      .map((p) => p.number)
      .sort((a, b) => a - b),
  );

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    const read = parseSix(six);
    if ("error" in read) {
      note = read.error;
      return;
    }
    const s = Number(setter);
    if (!setter.trim() || !read.court.includes(s)) {
      note = "The setter must be one of the six.";
      return;
    }
    note = "";
    const ok = await ui.apply(`${team.name} lineup entered`, () =>
      backend.setLineup(side, read.court, s),
    );
    if (ok) {
      six = "";
      setter = "";
    }
  }
</script>

<form class="sub-form" onsubmit={submit}>
  <div class="title"><SideTag {side} /> {team.name}: lineup for set {snap.set}</div>
  <div class="pick">
    <label
      >Zones 1–6
      <input class="lineup-six" bind:value={six} placeholder="8 12 13 11 3 14" autocomplete="off" />
    </label>
    <label
      >Setter
      <input class="lineup-setter" bind:value={setter} placeholder="8" autocomplete="off" />
    </label>
    <button class="btn primary" type="submit">Enter lineup</button>
  </div>
  <div class="sub-note" class:bad={note !== ""}>
    {note || `Players: ${players.join(" ")}`}
  </div>
</form>
