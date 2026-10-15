<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "./lib/backend";
  import ChecksPanel from "./lib/components/ChecksPanel.svelte";
  import CodeStream from "./lib/components/CodeStream.svelte";
  import CommandBar from "./lib/components/CommandBar.svelte";
  import Court from "./lib/components/Court.svelte";
  import Rosters from "./lib/components/Rosters.svelte";
  import StartScreen from "./lib/components/StartScreen.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import VideoPanel from "./lib/components/VideoPanel.svelte";
  import { run } from "./lib/actions";
  import { actionFor, keyString, usableWhileTyping } from "./lib/settings";
  import { ui } from "./lib/ui.svelte";

  let loaded = $state(false);

  // The season's keys, from anywhere. On the command line only keys
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
      await ui.loadKeymap();
    } catch (e) {
      // The last season moved or broke: start from the start screen.
      console.error(e);
    }
    loaded = true;
  });
</script>

<svelte:window {onkeydown} />

{#if ui.snapshot}
  {@const snap = ui.snapshot}
  <div class="app">
    <TopBar {snap} />
    <main class="work">
      <div class="stage">
        <VideoPanel />
        <div class="mid">
          <CodeStream {snap} />
          <Rosters {snap} />
        </div>
        <CommandBar />
      </div>
      <div class="right">
        <Court {snap} />
        <ChecksPanel {snap} />
      </div>
    </main>
  </div>
{:else if loaded}
  <StartScreen />
{/if}
