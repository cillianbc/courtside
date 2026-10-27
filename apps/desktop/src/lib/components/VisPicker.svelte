<!--
  Picks a match from FIVB VIS (issue #60): a tournament, then one of its
  matches. Its teams and registered players then go to the same review as
  a scoresheet.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import type { SheetChoice, SheetPreview } from "../scoresheet";
  import type { Snapshot } from "../snapshot";
  import { findTournaments, when, type VisMatch, type VisTournament } from "../vis";

  let {
    onpick,
    oncancel,
  }: {
    onpick: (preview: SheetPreview, create: (choice: SheetChoice) => Promise<Snapshot>) => void;
    oncancel: () => void;
  } = $props();

  let all = $state.raw<VisTournament[] | null>(null);
  let query = $state(String(new Date().getFullYear()));
  let tournament = $state.raw<VisTournament | null>(null);
  let matches = $state.raw<VisMatch[] | null>(null);
  let error = $state("");
  let busy = $state(false);
  const shown = $derived(all ? findTournaments(all, query).slice(0, 60) : []);

  onMount(() => {
    backend.visTournaments().then(
      (t) => (all = t),
      (e) => (error = String(e)),
    );
  });

  async function pickTournament(t: VisTournament) {
    error = "";
    tournament = t;
    matches = null;
    try {
      matches = await backend.visMatches(t.no);
    } catch (e) {
      error = String(e);
    }
  }

  async function pickMatch(m: VisMatch) {
    if (!tournament) return;
    const t = tournament;
    error = "";
    busy = true;
    try {
      const preview = await backend.visPreview(t, m);
      onpick(preview, (choice) => backend.matchFromVis(t, m, choice));
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="panel sheet-review" aria-label="FIVB VIS">
  <h2>New match from FIVB VIS</h2>
  <p class="sub-note">
    Courtside asks VIS for its public tournament, match and player lists; nothing about you or your
    matches is sent.
  </p>
  {#if error}<p class="error">{error}</p>{/if}
  {#if !tournament}
    <label class="vis-search"
      >Tournament <input bind:value={query} placeholder="name, code or year" />
    </label>
    {#if all === null && !error}
      <p class="sub-note">Asking VIS…</p>
    {:else}
      <div class="match-list">
        {#each shown as t (t.no)}
          <button type="button" class="match-item" onclick={() => pickTournament(t)}>
            <span>{t.name}</span>
            <span class="meta">{t.code}</span>
            <span class="meta">{t.startDate ?? t.season}</span>
          </button>
        {:else}
          <p class="empty-note">No tournament matches “{query}”.</p>
        {/each}
      </div>
    {/if}
  {:else}
    <p>
      <strong>{tournament.name}</strong>
      <button class="link" type="button" onclick={() => (tournament = null)}
        >Another tournament</button
      >
    </p>
    {#if matches === null && !error}
      <p class="sub-note">Asking VIS…</p>
    {:else if matches}
      <div class="match-list">
        {#each matches as m (m.no)}
          <button type="button" class="match-item" disabled={busy} onclick={() => pickMatch(m)}>
            <span>{m.teams[0].name} – {m.teams[1].name}</span>
            <span class="meta">{when(m)}</span>
            <span class="meta">{m.hall}{m.city ? `, ${m.city}` : ""}</span>
            <span class="meta">{m.noInTournament ? `match ${m.noInTournament}` : ""}</span>
          </button>
        {:else}
          <p class="empty-note">VIS lists no matches for this tournament.</p>
        {/each}
      </div>
    {/if}
  {/if}
  <div class="actions">
    <button class="btn" type="button" onclick={oncancel}>Cancel</button>
  </div>
</section>
