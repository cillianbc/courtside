<script lang="ts">
  import * as backend from "../backend";
  import { rosterProblems, type PlayerDetail, type TeamDetail } from "../teams";
  import RosterGrid from "./RosterGrid.svelte";

  let { team, onclose }: { team: TeamDetail | null; onclose: (saved: boolean) => void } = $props();

  // Edits happen on copies; nothing is saved until Save.
  // svelte-ignore state_referenced_locally
  let form = $state({
    name: team?.name ?? "",
    code: team?.code ?? "",
    coach: team?.coach ?? "",
    assistant: team?.assistant ?? "",
  });
  // svelte-ignore state_referenced_locally
  let rows = $state<PlayerDetail[]>(structuredClone(team?.players ?? []));
  let error = $state("");
  let saving = $state(false);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = "";
    if (!form.name.trim()) {
      error = "Enter the team's name.";
      return;
    }
    const problems = rosterProblems(rows);
    if (problems.size) {
      error = [...problems.values()][0] ?? "";
      return;
    }
    saving = true;
    try {
      await backend.saveTeam(team?.id ?? null, form, $state.snapshot(rows));
      onclose(true);
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }
</script>

<form class="panel team-editor" onsubmit={save}>
  <h2>{team ? `Edit ${team.name}` : "New team"}</h2>
  <div class="pick">
    <label class="grow">Name <input bind:value={form.name} placeholder="Harbour City VC" /></label>
    <label
      >Code
      <input
        class="short"
        bind:value={form.code}
        placeholder="auto"
        title="Left empty: made from the name"
      />
    </label>
    <label class="grow">Coach <input bind:value={form.coach} /></label>
    <label class="grow">Assistant <input bind:value={form.assistant} /></label>
  </div>
  <RosterGrid bind:rows />
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn primary" type="submit" disabled={saving}>Save team</button>
    <button class="btn" type="button" onclick={() => onclose(false)}>Cancel</button>
  </div>
</form>
