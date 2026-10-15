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
  import { ui } from "./lib/ui.svelte";

  let loaded = $state(false);

  // Undo and redo from anywhere; the command line's own text undo is
  // only lost while it holds text the scout hasn't entered.
  function onkeydown(e: KeyboardEvent) {
    if (!ui.snapshot || !(e.ctrlKey || e.metaKey)) return;
    const key = e.key.toLowerCase();
    const redo = (key === "z" && e.shiftKey) || key === "y";
    if (key !== "z" && key !== "y") return;
    if (ui.code.trim() && document.activeElement?.classList.contains("code-input")) return;
    e.preventDefault();
    void (redo ? ui.redo() : ui.undo());
  }
  onMount(async () => {
    try {
      ui.snapshot = await backend.resume();
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
