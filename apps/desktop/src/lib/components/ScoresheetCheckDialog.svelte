<!-- Where the coded match and its scoresheet disagree (issue #57). -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import type { SheetCheck } from "../scoresheet";
  import { ui } from "../ui.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let check = $state<SheetCheck | null>(null);
  let error = $state("");

  // The match video's native view would hide the dialog.
  onMount(() => {
    ui.covering += 1;
    return () => (ui.covering -= 1);
  });

  onMount(() => {
    dialog?.showModal();
    backend.scoresheetCheck().then(
      (c) => (check = c),
      (e) => (error = String(e)),
    );
  });
</script>

<dialog class="roster-dialog" bind:this={dialog} {onclose}>
  <h2>Check against the scoresheet</h2>
  {#if error}
    <p class="error">{error}</p>
  {:else if check === null}
    <p class="sub-note">Reading…</p>
  {:else if check.sets.length === 0}
    <p class="sub-note">No set is coded yet.</p>
  {:else if check.differences.length === 0}
    <p class="sub-note">
      Set{check.sets.length === 1 ? "" : "s"}
      {check.sets.join(", ")}: the sixes, scores and substitutions agree with the sheet.
    </p>
  {:else}
    <p class="sub-note">
      Compared set{check.sets.length === 1 ? "" : "s"}
      {check.sets.join(", ")}. Substitution scores are the substituting team's own first, as the
      sheet writes them. Either the coding or the sheet may be wrong.
    </p>
    <ul class="sheet-diffs">
      {#each check.differences as d, i (i)}<li>{d}</li>{/each}
    </ul>
  {/if}
  <div class="actions">
    <button class="btn" type="button" onclick={() => dialog?.close()}>Close</button>
  </div>
</dialog>
