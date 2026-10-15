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
  onMount(async () => {
    ui.snapshot = await backend.snapshot();
    loaded = true;
  });
</script>

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
