<!-- Asked once per computer when the build can send crash reports; the
     answer can be changed in Settings. -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import { ui } from "../ui.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  onMount(() => {
    ui.covering += 1;
    dialog?.showModal();
    return () => (ui.covering -= 1);
  });

  async function answer(yes: boolean) {
    try {
      await backend.setPrivacy(yes);
    } finally {
      dialog?.close();
    }
  }
</script>

<dialog
  class="report-dialog"
  bind:this={dialog}
  {onclose}
  oncancel={(e) => e.preventDefault()}
  aria-labelledby="consent-title"
>
  <h2 id="consent-title">Send crash reports?</h2>
  <p>
    When Courtside goes wrong, it can send us what failed and where in its code, with the app
    version and your system, so we can fix it.
  </p>
  <p class="hint">
    Never your codes, matches, players' names or file names. You can change this in Settings → Crash
    reports.
  </p>
  <div class="actions">
    <button class="btn primary" type="button" onclick={() => answer(true)}
      >Send crash reports</button
    >
    <button class="btn" type="button" onclick={() => answer(false)}>Don't send</button>
  </div>
</dialog>
