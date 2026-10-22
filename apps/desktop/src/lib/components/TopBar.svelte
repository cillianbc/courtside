<script lang="ts">
  import { exportDvw } from "../actions";
  import { withKey } from "../kbd";
  import * as backend from "../backend";
  import type { Snapshot } from "../snapshot";
  import { ui, type Theme } from "../ui.svelte";
  import Icon from "./Icon.svelte";
  import SideTag from "./SideTag.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const themes: [Theme, "sun" | "moon" | "system", string][] = [
    ["light", "sun", "Light"],
    ["dark", "moon", "Dark"],
    ["system", "system", "Follow the system"],
  ];
  // Sets won so far, from the finished sets.
  const won = $derived(
    snap.sets
      .filter((s) => s.number !== snap.set)
      .reduce(
        (w, s) => {
          if (s.score.home > s.score.visiting) w.home++;
          else if (s.score.visiting > s.score.home) w.visiting++;
          return w;
        },
        { home: 0, visiting: 0 },
      ),
  );
</script>

<!-- Three columns (v6 T5): the menu, the match in the middle, the actions on
     the right. Long names shorten, so Export and the theme stay at 1024 px. -->
<header class="bar" aria-label="Match">
  <div class="bar-left">
    <button
      class="btn"
      type="button"
      title="Back to the main menu: matches, teams, players, settings. The match is saved."
      disabled={!backend.inTauri}
      onclick={() => {
        ui.snapshot = null;
        ui.cancelEdit();
      }}><Icon name="menu" />Menu</button
    >
  </div>
  <div class="bar-match">
    <SideTag side="home" />
    <span class="team-name home" title={snap.teams.home.name}>{snap.teams.home.name}</span>
    <div class="score" aria-live="polite">
      <span class="serve-dot" title="Serving" class:off={snap.serving !== "home"}></span>
      <span class="h">{snap.score.home}</span><span class="sep">:</span><span class="v"
        >{snap.score.visiting}</span
      >
      <span class="serve-dot" title="Serving" class:off={snap.serving !== "visiting"}></span>
    </div>
    <span class="team-name away" title={snap.teams.visiting.name}>{snap.teams.visiting.name}</span>
    <SideTag side="visiting" />
    <span class="set-info">
      {#if snap.set === snap.goldenSet}
        Golden set · <b>first to 15</b>
      {:else}
        Set <b>{snap.set}</b> · sets <b class="num">{won.home}:{won.visiting}</b>
      {/if}
    </span>
  </div>
  <div class="bar-right">
    <div class="seg" role="group" aria-label="Layout">
      {#each [["watch", "Video", "The video fills the window beside one column"], ["coding", "Coding", "Video at 16:9, codes under it, the court and Checks beside"]] as [id, label, title] (id)}
        <button
          type="button"
          title={withKey(title ?? "", "layout")}
          aria-pressed={ui.layout === id}
          onclick={() => ui.setLayout(id === "watch" ? "watch" : "coding")}>{label}</button
        >
      {/each}
    </div>
    <button
      class="btn"
      type="button"
      title={withKey("Export the match as a .dvw file", "exportDvw")}
      disabled={!backend.inTauri}
      onclick={exportDvw}><Icon name="export" />Export</button
    >
    <div class="seg icons" role="group" aria-label="Theme">
      {#each themes as [id, icon, label] (id)}
        <button
          type="button"
          title={label}
          aria-label={label}
          aria-pressed={ui.theme === id}
          onclick={() => ui.setTheme(id)}><Icon name={icon} /></button
        >
      {/each}
    </div>
    <div class="seg" role="group" aria-label="Contrast">
      <button
        type="button"
        title="Boost contrast, over light or dark (this computer)"
        aria-pressed={ui.boost}
        onclick={() => ui.setBoost(!ui.boost)}><Icon name="boost" />Boost</button
      >
    </div>
  </div>
</header>
