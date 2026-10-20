<script lang="ts">
  import * as backend from "../backend";
  import { showCode } from "../jump";
  import type { IssueView, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  let { snap }: { snap: Snapshot } = $props();

  const FLAG = { error: "err", warning: "warn", info: "info" } as const;
  // Knock-ons sit under the fault they follow from, not in the list.
  const roots = $derived(snap.issues.filter((i) => !i.causeId).map((i) => i.eventId));
  const issues = $derived(
    snap.issues
      .filter((i) => !i.causeId || !roots.includes(i.causeId))
      .sort((a, b) => b.index - a.index),
  );
  const following = $derived.by(() => {
    const by: Record<string, IssueView[]> = {};
    for (const i of snap.issues) {
      if (i.causeId) (by[i.causeId] ??= []).push(i);
    }
    return by;
  });
  let open = $state<string | null>(null);
  const counts = $derived({
    error: snap.issues.filter((i) => i.severity === "error").length,
    warning: snap.issues.filter((i) => i.severity === "warning").length,
    info: snap.issues.filter((i) => i.severity === "info").length,
  });

  function show(issue: IssueView) {
    showCode(snap, issue.eventId);
  }

  function fixRotations(issue: IssueView) {
    void ui.apply("Rotations written again from here", () => backend.fixRotations(issue.eventId));
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
        {@const later = following[i.eventId] ?? []}
        {@const key = i.eventId + i.message}
        <div class="issue-wrap">
          <button type="button" class="issue" onclick={() => show(i)}>
            <span class="flag {FLAG[i.severity]}">{i.severity}</span>
            <span class="msg">{i.message}</span>
            <span class="where">code {i.index + 1} · {i.rule}</span>
          </button>
          {#if later.length || (i.rule === "rotation" && i.severity !== "info")}
            <div class="issue-tools">
              {#if later.length}
                <button
                  type="button"
                  class="link"
                  aria-expanded={open === key}
                  onclick={() => (open = open === key ? null : key)}
                  >{later.length} later {later.length === 1 ? "issue follows" : "issues follow"} from
                  this</button
                >
              {/if}
              {#if i.rule === "rotation" && i.severity !== "info"}
                <button
                  type="button"
                  class="btn"
                  title="Write the app's rotation codes again from this rally to the end of the set. Rotations you typed stay."
                  onclick={() => fixRotations(i)}>Fix rotations from here</button
                >
              {/if}
            </div>
          {/if}
          {#if open === key}
            {#each [...later].sort((a, b) => a.index - b.index) as k (k.eventId + k.message)}
              <button type="button" class="issue knock" onclick={() => show(k)}>
                <span class="flag info">follows</span>
                <span class="msg">{k.message}</span>
                <span class="where">code {k.index + 1} · {k.rule}</span>
              </button>
            {/each}
          {/if}
        </div>
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
