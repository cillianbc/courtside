<!-- This match's own roster for one team: late additions, a corrected
     number. The team's season roster is not changed. -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import type { Side } from "../snapshot";
  import { rosterProblems, type PlayerDetail } from "../teams";
  import { ui } from "../ui.svelte";
  import RosterGrid from "./RosterGrid.svelte";

  let { side, name, onclose }: { side: Side; name: string; onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let rows = $state<PlayerDetail[]>([]);
  let error = $state("");

  // The match video's native view would hide the dialog.
  onMount(() => {
    ui.covering += 1;
    return () => (ui.covering -= 1);
  });

  onMount(() => {
    dialog?.showModal();
    backend.matchRoster(side === "home").then(
      (r) => (rows = r),
      (e) => (error = String(e)),
    );
  });

  async function save() {
    const problems = rosterProblems(rows);
    if (problems.size) {
      error = [...problems.values()][0] ?? "";
      return;
    }
    const ok = await ui.apply(`${name} roster for this match saved`, () =>
      backend.saveMatchRoster(side === "home", $state.snapshot(rows)),
    );
    if (ok) onclose();
    else error = ui.result?.text ?? "";
  }
</script>

<dialog class="roster-dialog" bind:this={dialog} {onclose}>
  <h2>{name}: roster for this match</h2>
  <p class="sub-note">Changes here apply to this match only, not to the team's season roster.</p>
  <RosterGrid bind:rows details={false} />
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn primary" type="button" onclick={save}>Save roster</button>
    <button class="btn" type="button" onclick={() => dialog?.close()}>Cancel</button>
  </div>
</dialog>
