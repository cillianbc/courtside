<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import * as backend from "../backend";
  import type { Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap }: { snap: Snapshot } = $props();

  async function exportDvw() {
    try {
      const name = `${snap.teams.home.name} - ${snap.teams.visiting.name}.dvw`;
      const path = await save({
        defaultPath: name,
        filters: [{ name: "DataVolley scout", extensions: ["dvw"] }],
      });
      if (!path) return;
      await backend.exportDvw(path);
      ui.result = { text: `Exported ${path}.`, bad: false };
    } catch (e) {
      ui.result = { text: String(e), bad: true };
    }
  }

  function toggleTheme() {
    const root = document.documentElement;
    const current = root.getAttribute("data-theme");
    const dark = current
      ? current === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.setAttribute("data-theme", dark ? "light" : "dark");
  }
</script>

<header class="bar" aria-label="Match">
  <div class="teams">
    <span class="team-name home"><SideTag side="home" />{snap.teams.home.name}</span>
    <div class="score" aria-live="polite">
      <span class="serve-dot" title="Serving" hidden={snap.serving !== "home"}></span>
      <span class="h">{snap.score.home}</span><span class="sep">:</span><span class="v"
        >{snap.score.visiting}</span
      >
      <span class="serve-dot" title="Serving" hidden={snap.serving !== "visiting"}></span>
    </div>
    <span class="team-name away">{snap.teams.visiting.name}<SideTag side="visiting" /></span>
  </div>
  <div class="sets" aria-label="Sets">
    {#each snap.sets as s (s.number)}
      <span class="set-chip" class:now={s.number === snap.set}
        >S{s.number} {s.score.home}:{s.score.visiting}</span
      >
    {/each}
  </div>
  <div class="saved">
    <span>Saved</span><b>event {snap.events}</b>
    <button class="theme-btn" type="button" disabled={!backend.inTauri} onclick={exportDvw}
      >Export .dvw</button
    >
    <button class="theme-btn" type="button" onclick={toggleTheme}>Theme</button>
  </div>
</header>
