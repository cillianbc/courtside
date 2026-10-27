<!--
  What a scoresheet says, and how it meets the season, before a match is
  made from it (issue #57). The scout picks which season team each sheet
  team is, or adds it.
-->
<script lang="ts">
  import * as backend from "../backend";
  import { scoreText, teamPlan, type SheetChoice, type SheetPreview } from "../scoresheet";
  import type { TeamDetail } from "../teams";
  import { ui } from "../ui.svelte";

  let {
    path,
    preview,
    teams,
    onclose,
  }: { path: string; preview: SheetPreview; teams: TeamDetail[]; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let chosen = $state<[string | null, string | null]>([
    preview.teams[0].teamId,
    preview.teams[1].teamId,
  ]);
  let error = $state("");
  let busy = $state(false);
  const sheet = $derived(preview.sheet);
  // Each sheet team with how it meets the season, home first.
  const rows = $derived(
    sheet.teams.map((st, i) => ({ i: i as 0 | 1, st, m: preview.teams[i as 0 | 1] })),
  );
  const sides = ["Home", "Away"] as const;

  async function create() {
    error = "";
    if (chosen[0] !== null && chosen[0] === chosen[1]) {
      error = "Pick two different teams.";
      return;
    }
    busy = true;
    try {
      const choice: SheetChoice = { teamIds: [chosen[0], chosen[1]] };
      ui.snapshot = await backend.matchFromScoresheet(path, choice);
      await ui.loadKeymap();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="panel sheet-review" aria-label="Scoresheet">
  <h2>New match from a scoresheet</h2>
  <p class="sub-note">
    {[sheet.competition, sheet.matchNumber && `match ${sheet.matchNumber}`]
      .filter(Boolean)
      .join(" · ")}
    {#if sheet.date}· {sheet.date} {sheet.time}{/if}
    {#if sheet.hall}· {sheet.hall}{sheet.city ? `, ${sheet.city}` : ""}{/if}
  </p>

  <div class="sheet-teams">
    {#each rows as { i, st, m } (i)}
      <div class="sheet-team">
        <span class="role">{sides[i]}: <strong>{st.name}</strong></span>
        <label
          >Season team
          <select
            value={chosen[i] ?? ""}
            onchange={(e) => (chosen[i] = e.currentTarget.value || null)}
          >
            <option value="">Add {st.name} as a new team</option>
            {#each teams as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
          </select>
        </label>
        <span class="sub-note">{teamPlan(m, st, chosen[i])}</span>
        {#if chosen[i] !== null && chosen[i] === m.teamId && m.differing.length}
          <span class="sub-note bad"
            >Names differ (the roster is kept): {m.differing.join("; ")}.</span
          >
        {/if}
      </div>
    {/each}
  </div>

  <table class="list">
    <thead>
      <tr
        ><th>Set</th><th>{sheet.teams[0].name}</th><th>{sheet.teams[1].name}</th><th
          >Serves first</th
        ><th class="num">Score</th><th class="num">Substitutions</th></tr
      >
    </thead>
    <tbody>
      {#each sheet.sets as s (s.number)}
        <tr>
          <td class="num">{s.number}</td>
          <td class="mono">{s.lineups[0].join(" ")}</td>
          <td class="mono">{s.lineups[1].join(" ")}</td>
          <td>{s.servedFirst === null ? "–" : (sheet.teams[s.servedFirst]?.name ?? "–")}</td>
          <td class="num">{scoreText(s.score)}</td>
          <td class="num">{s.substitutions.length}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="sub-note">
    Each set's six fills the lineup screen when the set starts. Once coded, the match can be checked
    against the sheet's substitutions and scores.
    {#if sheet.referees.length}Referees: {sheet.referees.join(", ")}.{/if}
  </p>
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn primary" type="button" disabled={busy} onclick={create}>Create match</button>
    <button class="btn" type="button" onclick={onclose}>Cancel</button>
  </div>
</section>
