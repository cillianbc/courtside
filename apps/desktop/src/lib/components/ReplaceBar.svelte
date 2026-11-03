<script lang="ts">
  import * as backend from "../backend";
  import { pickCode } from "../jump";
  import type { Followed, Rule, RuleChange, Snapshot } from "../snapshot";
  import { ui } from "../ui.svelte";

  // Find and replace (#25): one change to every code the search finds,
  // previewed before it's made; one undo takes it back. Codes that change
  // with them (a pair's other half, a rally's ending) are listed before and
  // after, as the scout asked.
  let { snap }: { snap: Snapshot } = $props();

  const RATINGS = ["#", "+", "!", "-", "/", "="];

  let mode = $state<"replace" | "rating" | "after" | "before">("replace");
  let text = $state("");
  let rating = $state("+");
  let team = $state<"same" | "other">("other");
  let changes = $state<RuleChange[]>([]);
  let problem = $state<string | null>(null);
  let listOpen = $state(false);
  // Codes unticked in the preview, by event id: forgotten when the search
  // or the rule changes, kept while only the match does.
  let left = $state<string[]>([]);

  const rule = $derived<Rule>(
    mode === "replace"
      ? { kind: "replace", with: text }
      : mode === "rating"
        ? { kind: "rating", evaluation: rating }
        : { kind: "add", code: text, place: mode, team },
  );
  const ready = $derived(
    mode === "replace" ? !!ui.findFilter.text : mode === "rating" || !!text.trim(),
  );

  const question = $derived(JSON.stringify([ui.findFilter, rule]));
  let asked = "";
  $effect(() => {
    if (question !== asked) {
      asked = question;
      left = [];
    }
  });

  $effect(() => {
    void snap;
    const f = $state.snapshot(ui.findFilter);
    const r = $state.snapshot(rule);
    const l = $state.snapshot(left);
    if (!ready) {
      changes = [];
      problem = null;
      return;
    }
    backend.previewRule(f, r, l).then(
      (c) => {
        changes = c;
        problem = null;
      },
      (e) => {
        changes = [];
        problem = String(e);
      },
    );
  });

  const done = $derived(changes.filter((c) => !c.error && !c.left));
  const skipped = $derived(changes.filter((c) => c.error).length);
  const unticked = $derived(changes.filter((c) => c.left).length);
  const following = $derived(done.flatMap((c) => c.followed));
  const verb = $derived(mode === "replace" ? "Replace" : mode === "rating" ? "Rate" : "Add");

  function plural(n: number, one: string, many = `${one}s`): string {
    return `${n} ${n === 1 ? one : many}`;
  }

  /** "2 paired codes, 1 point": what followed, by kind. */
  function byKind(f: Followed[]): string {
    const names: Record<Followed["kind"], [string, string]> = {
      paired: ["paired code", "paired codes"],
      point: ["point", "points"],
      green: ["green code", "green codes"],
      rotation: ["rotation", "rotations"],
      other: ["other code", "other codes"],
    };
    return (Object.keys(names) as Followed["kind"][])
      .map((k) => [k, f.filter((x) => x.kind === k).length] as const)
      .filter(([, n]) => n > 0)
      .map(([k, n]) => plural(n, ...names[k]))
      .join(", ");
  }

  function follows(f: Followed): string {
    if (f.before && f.after) return `${f.before} → ${f.after}`;
    return f.after ? `+ ${f.after}` : `− ${f.before ?? ""}`;
  }

  function tick(id: string, on: boolean) {
    left = on ? left.filter((x) => x !== id) : [...left, id];
  }

  async function apply() {
    const n = done.length;
    const what =
      mode === "replace"
        ? `Replaced ${ui.findFilter.text} with ${text} in ${plural(n, "code")}`
        : mode === "rating"
          ? `Rated ${plural(n, "code")} ${rating}`
          : `Added ${plural(n, "code")}`;
    let applied: RuleChange[] = [];
    const ok = await ui.apply(
      what,
      async () => {
        const r = await backend.applyRule(
          $state.snapshot(ui.findFilter),
          $state.snapshot(rule),
          $state.snapshot(left),
        );
        applied = r.changes;
        return r.snapshot;
      },
      true,
    );
    if (!ok) return;
    const followed = applied.flatMap((c) => c.followed);
    if (!followed.length) return;
    ui.followed = followed.flatMap((f) => (f.id ? [f.id] : []));
    const shown = ui.followed.length > 0;
    ui.result = {
      text: `${what} · ${plural(followed.length, "other code", "others")} changed with them: ${byKind(followed)}.`,
      bad: false,
      action: shown
        ? {
            label: "Show",
            run: () => {
              ui.followedShown = true;
              const first = ui.followed[0];
              if (first && ui.snapshot) pickCode(ui.snapshot, first);
            },
          }
        : undefined,
    };
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Enter" && done.length) {
      e.preventDefault();
      void apply();
    } else if (e.key === "Escape") {
      e.preventDefault();
      ui.replacing = false;
    }
  }
</script>

<div class="replace-bar" role="group" aria-label="Find and replace">
  <div class="replace-row">
    <select bind:value={mode} aria-label="What to do with each code found">
      <option value="replace">Replace with</option>
      <option value="rating">Set rating to</option>
      <option value="after">Add after</option>
      <option value="before">Add before</option>
    </select>
    {#if mode === "rating"}
      <select bind:value={rating} aria-label="Rating" class="mono" onkeydown={key}>
        {#each RATINGS as r (r)}<option value={r}>{r}</option>{/each}
      </select>
      <span class="replace-note grow">each touch the search finds</span>
    {:else}
      <input
        bind:value={text}
        type="text"
        spellcheck="false"
        autocomplete="off"
        placeholder={mode === "replace" ? "Replacement text" : "Code without the team: $$D, 5D-"}
        aria-label={mode === "replace" ? "Replacement text" : "Code to add"}
        title={mode === "replace"
          ? "Replaces the Find text in every code found, as if you had modified each one."
          : "Added next to every touch found, for its team or the other one; $$ for no particular player."}
        onkeydown={key}
      />
    {/if}
    {#if mode === "after" || mode === "before"}
      <select bind:value={team} aria-label="Which team the code is for">
        <option value="other">other team</option>
        <option value="same">same team</option>
      </select>
    {/if}
    <button
      class="btn small primary"
      type="button"
      disabled={!done.length}
      title="One undo takes it all back"
      onclick={apply}>{verb} {done.length || ""}</button
    >
  </div>
  {#if problem}
    <p class="replace-note bad">{problem}</p>
  {:else if mode === "replace" && !ui.findFilter.text}
    <p class="replace-note">Type the text to replace in Find.</p>
  {:else if changes.length}
    <button
      class="replace-note link"
      type="button"
      aria-expanded={listOpen}
      onclick={() => (listOpen = !listOpen)}
      >{done.length} to change{#if unticked}, {unticked} unticked{/if}{#if skipped}, {skipped}
        left as they are{/if}{#if following.length}
        · <span class="replace-follow"
          >{plural(following.length, "other code", "others")} change with them ({byKind(
            following,
          )})</span
        >{/if} · {listOpen ? "hide" : "show"}</button
    >
    {#if listOpen}
      <ul class="replace-list">
        {#each changes as c (c.id)}
          <li class:bad={!!c.error} class:left={c.left}>
            <input
              type="checkbox"
              checked={!c.left}
              disabled={!!c.error}
              aria-label="Change {c.code}"
              onchange={(e) => tick(c.id, e.currentTarget.checked)}
            />
            <button type="button" onclick={() => pickCode(snap, c.id)}>
              <code>{c.code}</code>
              {#if c.error}<span>{c.error}</span>{:else if c.left}<span>left out</span>{:else}→ <code
                  >{c.result.join(" ")}</code
                >{/if}
            </button>
            {#each c.followed as f, i (i)}
              <span class="replace-follow">follows: <code>{follows(f)}</code></span>
            {/each}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>
