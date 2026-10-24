<!-- Report a problem: what happened, optionally the match, written to a
     folder the scout emails us (diagnostics.rs). -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import { recentActions } from "../trail";
  import { ui } from "../ui.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let note = $state("");
  let includeMatch = $state(false);
  let busy = $state(false);
  let done = $state("");
  let error = $state("");

  // The match video's native view would hide the dialog.
  onMount(() => {
    ui.covering += 1;
    dialog?.showModal();
    return () => (ui.covering -= 1);
  });

  async function send() {
    busy = true;
    error = "";
    try {
      done = await backend.reportProblem(note, includeMatch, recentActions());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog class="report-dialog" bind:this={dialog} {onclose} aria-labelledby="report-title">
  <h2 id="report-title">Report a problem</h2>
  {#if done}
    <p>
      Your mail app should now have an email ready, and the report's folder is open beside it.
      Attach the files in the folder and send it.
    </p>
    <p class="hint"><code>{done}</code></p>
    <div class="actions">
      <button class="btn primary" type="button" onclick={() => dialog?.close()}>Done</button>
    </div>
  {:else}
    <label class="field"
      >What happened, and what did you expect?
      <textarea bind:value={note} rows="5" placeholder="I typed … and the log showed …"></textarea>
    </label>
    {#if ui.snapshot}
      <label class="inline"
        ><input type="checkbox" bind:checked={includeMatch} /> Attach this match (.dvw) so we can see
        the codes</label
      >
      <p class="hint">
        The match file has your players' names. Leave it out if you'd rather not send them.
      </p>
    {/if}
    <p class="hint">
      The report has the app version, your system, the last actions you took (their names, not your
      codes), what Checks found, and the app's log. It's saved in your Downloads folder; nothing is
      sent until you send the email.
    </p>
    {#if error}<p class="bad">{error}</p>{/if}
    <div class="actions">
      <button class="btn primary" type="button" disabled={busy} onclick={send}
        >{busy ? "Writing…" : "Make the report"}</button
      >
      <button class="btn" type="button" onclick={() => dialog?.close()}>Cancel</button>
    </div>
  {/if}
</dialog>
