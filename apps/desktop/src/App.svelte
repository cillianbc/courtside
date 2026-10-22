<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "./lib/backend";
  import ChecksPanel from "./lib/components/ChecksPanel.svelte";
  import ClipsPanel from "./lib/components/ClipsPanel.svelte";
  import CodeStream from "./lib/components/CodeStream.svelte";
  import CommandBar from "./lib/components/CommandBar.svelte";
  import Court from "./lib/components/Court.svelte";
  import LineupScreen from "./lib/components/LineupScreen.svelte";
  import PlayCourt from "./lib/components/PlayCourt.svelte";
  import StartScreen from "./lib/components/StartScreen.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import VideoPanel from "./lib/components/VideoPanel.svelte";
  import { run } from "./lib/actions";
  import { actionFor, keyString, usableWhileTyping } from "./lib/settings";
  import { applyTeamColours } from "./lib/teams";
  import { ui } from "./lib/ui.svelte";

  let loaded = $state(false);

  // The open match's team colours, on the colour tokens (v6 T13).
  $effect(() => {
    const t = ui.snapshot?.teams;
    applyTeamColours(t?.home.colour ?? "", t?.visiting.colour ?? "");
  });

  // A team without its six can't play: the lineup screen until both are in.
  const needsLineup = $derived(
    !!ui.snapshot &&
      !ui.snapshot.matchOver &&
      (ui.snapshot.needsLineup.home || ui.snapshot.needsLineup.visiting),
  );

  // This computer's keys, from anywhere. On the command line only keys
  // with Mod or Alt (or function keys) count, so typing never triggers
  // one; Undo there with text typed stays the text box's own undo.
  function onkeydown(e: KeyboardEvent) {
    if (!ui.snapshot) return;
    const keys = keyString(e);
    const action = actionFor(ui.keymap, keys);
    if (!action || !keys) return;
    const inField = e.target instanceof HTMLElement && e.target.closest("input, select, textarea");
    if (inField && !usableWhileTyping(keys)) return;
    const typing = document.activeElement?.classList.contains("code-input") && ui.code.trim();
    if (typing && (action === "undo" || action === "redo")) return;
    if (run(action)) e.preventDefault();
  }
  onMount(async () => {
    try {
      ui.snapshot = await backend.resume();
    } catch (e) {
      // The last season moved or broke: start from the start screen.
      console.error(e);
    }
    await ui.loadKeymap();
    loaded = true;
  });
</script>

<svelte:window {onkeydown} />

{#if ui.snapshot}
  {@const snap = ui.snapshot}
  <div class="app">
    <TopBar {snap} />
    {#if needsLineup || ui.lineupScreen !== null}
      <!-- Keyed by set: the screen fills its sixes when it opens. -->
      {@const set = needsLineup ? snap.set : (ui.lineupScreen ?? snap.set)}
      {#key set}<LineupScreen {snap} {set} />{/key}
    {:else}
      <main class="work {ui.layout}">
        <VideoPanel />
        {#if ui.clipsOpen}
          <ClipsPanel {snap} />
        {:else if ui.checksOpen && ui.layout === "watch"}
          <ChecksPanel {snap} column />
        {:else}
          <CodeStream {snap} />
        {/if}
        <Court {snap} />
        <CommandBar />
        <PlayCourt {snap} />
        <ChecksPanel {snap} />
      </main>
    {/if}
  </div>
{:else if loaded}
  <StartScreen />
{/if}
