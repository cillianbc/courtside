<script lang="ts">
  import { exportDvw } from "../actions";
  import { withKey } from "../kbd";
  import * as backend from "../backend";
  import type { Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap }: { snap: Snapshot } = $props();

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
  <button
    class="theme-btn menu-btn"
    type="button"
    title="Back to the main menu: matches, teams, players, settings. The match is saved."
    disabled={!backend.inTauri}
    onclick={() => {
      ui.snapshot = null;
      ui.cancelEdit();
    }}>◀ Main menu</button
  >
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
        >{s.number === snap.goldenSet ? "GS" : `S${s.number}`}
        {s.score.home}:{s.score.visiting}</span
      >
    {/each}
  </div>
  <div class="saved">
    <div class="layout-switch" role="group" aria-label="Layout">
      {#each [["coding", "Coding", "Video on top, codes and the court beside it"], ["watch", "Video", "The video fills the window beside one column"]] as [id, label, title] (id)}
        <button
          type="button"
          title={withKey(title ?? "", "layout")}
          aria-pressed={ui.layout === id}
          onclick={() => ui.setLayout(id === "watch" ? "watch" : "coding")}>{label}</button
        >
      {/each}
    </div>
    <span>Saved</span><b>event {snap.events}</b>
    <button
      class="theme-btn"
      type="button"
      title={withKey("Export the match as a .dvw file", "exportDvw")}
      disabled={!backend.inTauri}
      onclick={exportDvw}>Export .dvw</button
    >
    <button class="theme-btn" type="button" onclick={toggleTheme}>Theme</button>
  </div>
</header>
