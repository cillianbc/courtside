<script lang="ts">
  import * as backend from "../backend";
  import { showCode } from "../jump";
  import type { IssueView, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";

  // `column`: shown in place of the code log (Video layout, v6 T8).
  let { snap, column = false }: { snap: Snapshot; column?: boolean } = $props();
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

  // "Fix rotations from here" first says what it will do (v6 review):
  // the preview for one issue at a time, then the result with undo.
  let preview = $state<{ id: string; p: backend.FixPreview } | null>(null);
  let fixed = $state<backend.FixPreview | null>(null);
  async function previewFix(issue: IssueView) {
    try {
      preview = { id: issue.eventId, p: await backend.fixRotationsPreview(issue.eventId) };
    } catch (e) {
      ui.result = { text: String(e), bad: true };
    }
  }
  async function applyFix() {
    if (!preview) return;
    const { id, p } = preview;
    preview = null;
    if (await ui.apply("Rotations written again from here", () => backend.fixRotations(id)))
      fixed = p;
  }
  function rallies(p: backend.FixPreview): string {
    if (p.fromRally === null) return "";
    return p.fromRally === p.toRally
      ? ` in rally ${p.fromRally}`
      : ` in rallies ${p.fromRally}–${p.toRally}`;
  }

  function selectTab(tab: "checks" | "set") {
    ui.sideTab = tab;
  }
</script>

<section class="panel side" class:in-column={column} aria-label="Checks and set summary">
  <div class="tabs" role="tablist">
    {#if column}
      <button
        class="btn icon"
        type="button"
        aria-label="Back to the codes"
        title="Back to the codes"
        onclick={() => (ui.checksOpen = false)}><Icon name="left" /></button
      >
    {/if}
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
        {#if counts.error}<span class="count" title="Errors"
            ><Mark severity="error" />{counts.error}</span
          >{/if}
        {#if counts.warning}<span class="count" title="Warnings"
            ><Mark severity="warning" />{counts.warning}</span
          >{/if}
        {#if counts.info}<span class="count" title="Info"
            ><Mark severity="info" />{counts.info}</span
          >{/if}
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
      {#if column}<p class="back-note">Typing a code goes back to the log.</p>{/if}
      {#if fixed}
        <div class="done-note">
          <span
            ><b>Fixed.</b>
            {fixed.codes} rotation {fixed.codes === 1 ? "code" : "codes"} written{rallies(
              fixed,
            )}.</span
          >
          <button
            type="button"
            class="btn"
            onclick={() => {
              fixed = null;
              void ui.undo();
            }}><Icon name="undo" />Undo</button
          >
          <button type="button" class="btn icon" aria-label="Close" onclick={() => (fixed = null)}
            >×</button
          >
        </div>
      {/if}
      {#each issues as i (i.eventId + i.message)}
        {@const later = following[i.eventId] ?? []}
        {@const key = i.eventId + i.message}
        <div class="issue-wrap" class:root={i.severity === "error" && later.length > 0}>
          <button type="button" class="issue" onclick={() => show(i)}>
            <Mark severity={i.severity} />
            <span class="msg">{i.message}</span>
            <span class="where">code {i.index + 1} · {i.rule}</span>
          </button>
          {#if later.length || (i.rule === "rotation" && i.severity !== "info")}
            <div class="issue-tools">
              {#if later.length}
                <button
                  type="button"
                  class="link"
                  aria-expanded={open !== key}
                  onclick={() => (open = open === key ? null : key)}
                  >{later.length} later {later.length === 1 ? "issue follows" : "issues follow"} from
                  this</button
                >
              {/if}
              {#if i.rule === "rotation" && i.severity !== "info"}
                <button
                  type="button"
                  class="btn primary"
                  title="Write the app's rotation codes again from this rally to the end of the set. Rotations you typed stay."
                  onclick={() => void previewFix(i)}>Fix rotations from here</button
                >
              {/if}
            </div>
          {/if}
          {#if preview?.id === i.eventId}
            {@const p = preview.p}
            <div class="fix-preview">
              <b>Fix rotations from here will:</b>
              <ul>
                <li>
                  write {p.codes} rotation {p.codes === 1 ? "code" : "codes"}{rallies(p)}
                </li>
                <li>leave {p.issuesAfter} of the {p.issuesBefore} issues</li>
                <li>keep the rotations you typed</li>
              </ul>
              <div class="fix-actions">
                <button type="button" class="btn primary" onclick={() => void applyFix()}
                  >Apply fix</button
                >
                <button type="button" class="btn" onclick={() => (preview = null)}>Cancel</button>
              </div>
            </div>
          {/if}
          {#if open !== key && later.length}
            {#each [...later].sort((a, b) => a.index - b.index) as k (k.eventId + k.message)}
              <button type="button" class="issue knock" onclick={() => show(k)}>
                <span class="knock-n">↳ {k.index + 1}</span>
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
        <span></span><span class="hd">{snap.teams.home.name}</span><span class="hd"
          >{snap.teams.visiting.name}</span
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
