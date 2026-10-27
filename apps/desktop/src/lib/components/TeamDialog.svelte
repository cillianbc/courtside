<!-- A match's team in the team editor, over the match (issue #49). Saving
     reopens the match, so a new name or colour shows at once. -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import type { TeamDetail } from "../teams";
  import { ui } from "../ui.svelte";
  import TeamEditor from "./TeamEditor.svelte";

  let { teamId, onclose }: { teamId: string; onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let team = $state.raw<TeamDetail | null>(null);
  let error = $state("");

  // The match video's native view would hide the dialog.
  onMount(() => {
    ui.covering += 1;
    return () => (ui.covering -= 1);
  });

  onMount(() => {
    dialog?.showModal();
    backend.teams().then(
      (all) => {
        team = all.find((t) => t.id === teamId) ?? null;
        if (!team) error = "This team isn't in the season any more.";
      },
      (e) => (error = String(e)),
    );
  });

  async function done(saved: TeamDetail | null) {
    if (saved && ui.snapshot) {
      const id = ui.snapshot.matchId;
      await ui.apply(`${saved.name} saved`, () => backend.openMatch(id));
    }
    dialog?.close();
  }
</script>

<dialog class="roster-dialog team-dialog" bind:this={dialog} {onclose}>
  {#if team}
    <TeamEditor {team} onclose={done} />
  {:else if error}
    <p class="error">{error}</p>
    <div class="actions">
      <button class="btn" type="button" onclick={() => dialog?.close()}>Close</button>
    </div>
  {/if}
</dialog>
