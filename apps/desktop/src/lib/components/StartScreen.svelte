<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import * as backend from "../backend";
  import type { MatchItem } from "../snapshot";
  import { ui } from "../ui.svelte";

  let list = $state<MatchItem[] | null>(null);
  let error = $state("");

  const SEASON = [{ name: "Courtside season", extensions: ["courtside"] }];

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
      list = await backend.matches();
    });

  const openSeason = () =>
    run(async () => {
      const path = await open({ filters: SEASON });
      if (typeof path !== "string") return;
      await backend.openSeason(path);
      list = await backend.matches();
    });

  const importFile = () =>
    run(async () => {
      const path = await open({ filters: [{ name: "DataVolley scout", extensions: ["dvw"] }] });
      if (typeof path !== "string") return;
      list = await backend.importMatch(path);
    });

  const openMatch = (id: string) =>
    run(async () => {
      ui.snapshot = await backend.openMatch(id);
    });
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
        <p class="empty-note">No matches yet. Import a .dvw file.</p>
      {/each}
    </div>
  {/if}
</main>
