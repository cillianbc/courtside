<script lang="ts">
  import { open, save } from "@tauri-apps/plugin-dialog";
  import * as backend from "../backend";
  import { describeImport, type TeamDetail } from "../teams";
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
  let note = $state("");

  async function exportFile(t: TeamDetail) {
    error = "";
    note = "";
    try {
      const path = await save({
        defaultPath: `${t.code || t.name}.sq`,
        filters: [
          { name: "DataVolley team (.sq), also read by VolleyStation", extensions: ["sq"] },
          { name: "VolleyStation team (.vst)", extensions: ["vst"] },
        ],
      });
      if (!path) return;
      await backend.exportTeam(t.id, path);
      note = `Wrote ${path}.`;
    } catch (e) {
      error = String(e);
    }
  }

  async function importFiles() {
    error = "";
    note = "";
    try {
      const picked = await open({
        multiple: true,
        filters: [
          { name: "Team file (DataVolley .sq, VolleyStation .vst)", extensions: ["sq", "vst"] },
        ],
      });
      const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
      const done: string[] = [];
      for (const path of paths) done.push(describeImport(await backend.importTeam(path)));
      note = done.join(" ");
      if (paths.length) await onchange();
    } catch (e) {
      error = String(e);
    }
  }

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
    <button class="btn" type="button" onclick={importFiles}>Import team file</button>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  {#if note}<p class="sub-note">{note}</p>{/if}
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
              <button class="btn" type="button" onclick={() => exportFile(t)}>Export</button>
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
