<script lang="ts">
  import type { IssueView, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const FLAG = { error: "err", warning: "warn", info: "info" } as const;
  const issues = $derived([...snap.issues].sort((a, b) => b.index - a.index));
  const counts = $derived({
    error: snap.issues.filter((i) => i.severity === "error").length,
    warning: snap.issues.filter((i) => i.severity === "warning").length,
    info: snap.issues.filter((i) => i.severity === "info").length,
  });

  function show(issue: IssueView) {
    const group = snap.groups.find((g) => g.rows.some((r) => r.id === issue.eventId));
    if (group) ui.shownSet = group.set === snap.set ? null : group.set;
    requestAnimationFrame(() => {
      const el = document.getElementById(`ev-${issue.eventId}`);
      if (!el) return;
      el.closest(".rally")?.classList.remove("closed");
      el.scrollIntoView({ block: "center" });
      el.classList.remove("hl");
      void el.offsetWidth;
      el.classList.add("hl");
    });
  }

  function selectTab(tab: "checks" | "set") {
    ui.sideTab = tab;
  }
</script>

<section class="panel side" aria-label="Checks and set summary">
  <div class="tabs" role="tablist">
    <button
      class="tab"
      role="tab"
      type="button"
      aria-selected={ui.sideTab === "checks"}
      tabindex={ui.sideTab === "checks" ? 0 : -1}
      onclick={() => selectTab("checks")}
    >
      Checks
      <span class="counts">
        {#if counts.error}<span class="flag err" title="Errors">{counts.error}</span>{/if}
        {#if counts.warning}<span class="flag warn" title="Warnings">{counts.warning}</span>{/if}
        <span class="flag info" title="Info">{counts.info}</span>
      </span>
    </button>
    <button
      class="tab"
      role="tab"
      type="button"
      aria-selected={ui.sideTab === "set"}
      tabindex={ui.sideTab === "set" ? 0 : -1}
      onclick={() => selectTab("set")}>This set</button
    >
  </div>
  {#if ui.sideTab === "checks"}
    <div class="tab-panel" role="tabpanel">
      {#each issues as i (i.eventId + i.message)}
        <button type="button" class="issue" class:knock={i.knockOn} onclick={() => show(i)}>
          <span class="flag {FLAG[i.severity]}">{i.severity}</span>
          <span class="msg">{i.message}</span>
          <span class="where">code {i.index + 1} · {i.rule}</span>
        </button>
      {:else}
        <div class="empty-note">No issues.</div>
      {/each}
    </div>
  {:else}
    <div class="tab-panel" role="tabpanel">
      <div class="tally">
        <span></span><span class="hd">{snap.teams.home.name.slice(0, 3).toUpperCase()}</span><span
          class="hd">{snap.teams.visiting.name.slice(0, 3).toUpperCase()}</span
        >
        <span>Points</span><span class="h">{snap.score.home}</span><span class="v"
          >{snap.score.visiting}</span
        >
        <span>Timeouts</span><span class="h">{snap.timeouts.home}</span><span class="v"
          >{snap.timeouts.visiting}</span
        >
        <span>Substitutions</span><span class="h">{snap.substitutions.home}</span><span class="v"
          >{snap.substitutions.visiting}</span
        >
      </div>
    </div>
  {/if}
</section>
