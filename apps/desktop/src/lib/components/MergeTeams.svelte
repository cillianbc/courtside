<script lang="ts">
  import * as backend from "../backend";
  import { guessPairs, type TeamDetail } from "../teams";

  let {
    from,
    teams,
    onclose,
  }: { from: TeamDetail; teams: TeamDetail[]; onclose: (merged: boolean) => void } = $props();

  let intoId = $state("");
  const into = $derived(teams.find((t) => t.id === intoId) ?? null);
  // Player of `from` → player of `into` they are; "" moves them across.
  let pairs = $state<Record<string, string>>({});
  let error = $state("");

  $effect(() => {
    if (into) pairs = Object.fromEntries(guessPairs(from.players, into.players));
  });

  async function merge() {
    if (!into) return;
    error = "";
    try {
      const list = Object.entries(pairs).filter(([, to]) => to) as [string, string][];
      await backend.mergeTeams(from.id, into.id, list);
      onclose(true);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="panel team-editor">
  <h2>Merge {from.name} into another team</h2>
  <p class="sub-note">
    Its {from.matches} match(es) move to the team you pick. Pair each player with who they are in that
    team, or leave them to join it as they are.
  </p>
  <label
    >Into
    <select bind:value={intoId}>
      <option value="">–</option>
      {#each teams.filter((t) => t.id !== from.id) as t (t.id)}
        <option value={t.id}>{t.name}</option>
      {/each}
    </select>
  </label>
  {#if into}
    <table class="list">
      <thead><tr><th>{from.name}</th><th>is in {into.name}</th></tr></thead>
      <tbody>
        {#each from.players as p (p.id)}
          <tr>
            <td>#{p.number} {p.lastName} {p.firstName}</td>
            <td>
              <select bind:value={pairs[p.id]}>
                <option value="">joins as a new player</option>
                {#each into.players as q (q.id)}
                  <option value={q.id}>#{q.number} {q.lastName} {q.firstName}</option>
                {/each}
              </select>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn primary" type="button" disabled={!into} onclick={merge}>Merge teams</button>
    <button class="btn" type="button" onclick={() => onclose(false)}>Cancel</button>
  </div>
</div>
