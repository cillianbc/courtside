<script lang="ts">
  import * as backend from "../backend";
  import type { TeamDetail } from "../teams";
  import MergeTeams from "./MergeTeams.svelte";
  import TeamEditor from "./TeamEditor.svelte";

  let { teams, onchange }: { teams: TeamDetail[]; onchange: () => Promise<void> } = $props();

  type View =
    | { kind: "list" }
    | { kind: "edit"; team: TeamDetail | null }
    | { kind: "merge"; team: TeamDetail };
  let view = $state<View>({ kind: "list" });
  let error = $state("");
  let confirmDelete = $state<string | null>(null);

  async function done(changed: boolean) {
    view = { kind: "list" };
    if (changed) await onchange();
  }

  async function remove(t: TeamDetail) {
    if (confirmDelete !== t.id) {
      confirmDelete = t.id;
      return;
    }
    confirmDelete = null;
    error = "";
    try {
      await backend.deleteTeam(t.id);
      await onchange();
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if view.kind === "edit"}
  <TeamEditor team={view.team} onclose={done} />
{:else if view.kind === "merge"}
  <MergeTeams from={view.team} {teams} onclose={done} />
{:else}
  <div class="actions">
    <button class="btn primary" type="button" onclick={() => (view = { kind: "edit", team: null })}
      >New team</button
    >
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  {#if teams.length}
    <table class="list">
      <thead>
        <tr
          ><th>Team</th><th>Code</th><th>Coach</th><th class="num">Players</th><th class="num"
            >Matches</th
          ><th></th></tr
        >
      </thead>
      <tbody>
        {#each teams as t (t.id)}
          <tr>
            <td>
              <button class="link" type="button" onclick={() => (view = { kind: "edit", team: t })}
                >{t.name}</button
              >
            </td>
            <td class="mono">{t.code}</td>
            <td>{t.coach}</td>
            <td class="num">{t.players.length}</td>
            <td class="num">{t.matches}</td>
            <td class="row-tools">
              <button class="btn" type="button" onclick={() => (view = { kind: "edit", team: t })}
                >Edit</button
              >
              <button
                class="btn"
                type="button"
                disabled={teams.length < 2}
                onclick={() => (view = { kind: "merge", team: t })}>Merge</button
              >
              <button
                class="btn"
                type="button"
                disabled={t.matches > 0}
                title={t.matches > 0 ? "Teams with matches can be merged, not deleted" : ""}
                onclick={() => remove(t)}
                >{confirmDelete === t.id ? "Really delete?" : "Delete"}</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="empty-note">No teams yet. Add one, or import a .dvw to take its teams.</p>
  {/if}
{/if}
