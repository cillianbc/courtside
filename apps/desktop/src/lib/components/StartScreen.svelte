<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import type { MatchItem, TeamItem } from "../snapshot";
  import { ui } from "../ui.svelte";

  let list = $state<MatchItem[] | null>(null);
  let teams = $state<TeamItem[]>([]);
  let error = $state("");
  let home = $state("");
  let visiting = $state("");
  let date = $state(new Date().toISOString().slice(0, 10));

  const SEASON = [{ name: "Courtside season", extensions: ["courtside"] }];

  async function refresh() {
    list = await backend.matches();
    teams = await backend.teams();
  }

  // A season may already be open (coming back from a match).
  onMount(() => {
    if (backend.inTauri) refresh().catch(() => {});
  });

  async function run(f: () => Promise<void>) {
    error = "";
    try {
      await f();
    } catch (e) {
      error = String(e);
    }
  }

  const newSeason = () =>
    run(async () => {
      const path = await save({ filters: SEASON, defaultPath: "season.courtside" });
      if (!path) return;
      const name = path.replace(/^.*[\\/]/, "").replace(/\.courtside$/, "");
      await backend.createSeason(path, name);
      await refresh();
    });

  const openSeason = () =>
    run(async () => {
      const path = await open({ filters: SEASON });
      if (typeof path !== "string") return;
      await backend.openSeason(path);
      await refresh();
    });

  const importFile = () =>
    run(async () => {
      const path = await open({ filters: [{ name: "DataVolley scout", extensions: ["dvw"] }] });
      if (typeof path !== "string") return;
      list = await backend.importMatch(path);
      teams = await backend.teams();
    });

  const openMatch = (id: string) =>
    run(async () => {
      ui.snapshot = await backend.openMatch(id);
    });

  const newMatch = (e: SubmitEvent) => {
    e.preventDefault();
    return run(async () => {
      if (!home || !visiting || home === visiting) {
        error = "Pick two different teams.";
        return;
      }
      ui.snapshot = await backend.newMatch(home, visiting, date || null);
    });
  };
</script>

<main class="start panel">
  <h1>Courtside</h1>
  <div class="actions">
    <button class="btn primary" type="button" onclick={openSeason}>Open season</button>
    <button class="btn" type="button" onclick={newSeason}>New season</button>
    {#if list}
      <button class="btn" type="button" onclick={importFile}>Import .dvw</button>
    {/if}
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  {#if list}
    <h2>New match</h2>
    {#if teams.length < 2}
      <p class="empty-note">
        Teams come from imported .dvw files for now. Import one to get its two teams and rosters.
      </p>
    {:else}
      <form class="sub-form" onsubmit={newMatch}>
        <div class="pick">
          <label
            >Home
            <select bind:value={home}>
              <option value="">–</option>
              {#each teams as t (t.id)}<option value={t.id}>{t.name} ({t.players})</option>{/each}
            </select>
          </label>
          <label
            >Away
            <select bind:value={visiting}>
              <option value="">–</option>
              {#each teams as t (t.id)}<option value={t.id}>{t.name} ({t.players})</option>{/each}
            </select>
          </label>
          <label>Date <input type="date" bind:value={date} /></label>
          <button class="btn primary" type="submit">Start coding</button>
        </div>
      </form>
    {/if}
    <h2>Matches</h2>
    <div class="match-list">
      {#each list as m (m.id)}
        <button type="button" class="match-item" onclick={() => openMatch(m.id)}>
          <span>{m.home} – {m.visiting}</span>
          <span class="meta">{m.date ?? ""}</span>
          <span class="meta">{m.sourceName ?? "coded here"}</span>
          <span class="meta">{m.events} events</span>
        </button>
      {:else}
        <p class="empty-note">No matches yet.</p>
      {/each}
    </div>
  {/if}
</main>
