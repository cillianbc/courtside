<!-- Season settings: what the command line fills in, the tables a .dvw
     carries, and the keys. -->
<script lang="ts">
  import { onMount } from "svelte";
  import * as backend from "../backend";
  import {
    EVALUATIONS,
    SKILLS,
    SKILL_NAMES,
    TYPES,
    conflicts,
    effectiveKeymap,
    type Settings,
  } from "../settings";
  import { ui } from "../ui.svelte";
  import KeymapEditor from "./KeymapEditor.svelte";
  import KeyTextEditor from "./KeyTextEditor.svelte";

  let settings = $state<Settings | null>(null);
  let error = $state("");
  let note = $state("");

  onMount(() => {
    backend.settings().then(
      // Seasons saved before the rules existed get them filled in by the backend.
      (s) => {
        // A backend from before remapped keys sends none.
        if (s) s.keyText ??= [];
        settings = s;
      },
      (e) => (error = String(e)),
    );
  });

  async function save() {
    if (!settings) return;
    error = "";
    note = "";
    const clash = [...conflicts(settings.keymap)];
    if (clash.length) {
      error = `${clash.join(", ")} is set for more than one action.`;
      return;
    }
    const actionKeys = new Set(effectiveKeymap(settings.keymap).map((k) => k.keys));
    const typed = settings.keyText.filter((k) => k.keys || k.text);
    for (const [i, k] of typed.entries()) {
      if (!k.keys || !k.text) {
        error = "Each remapped key needs a key and the text it types.";
        return;
      }
      if (typed.findIndex((o) => o.keys === k.keys) !== i || actionKeys.has(k.keys)) {
        error = `${k.keys} is set more than once.`;
        return;
      }
    }
    settings.keyText = typed;
    try {
      await backend.saveSettings($state.snapshot(settings));
      await ui.loadKeymap();
      note = "Saved. New matches take these tables; matches already created keep theirs.";
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#if settings}
  <section class="settings">
    <h2>Typing</h2>
    <p class="sub-note">
      What the command line fills in when a code leaves it out. A reception, block or dig takes the
      type of the touch it answers.
    </p>
    <div class="pick">
      <label
        >Skill when none is predicted
        <select bind:value={settings.typing.skill}>
          {#each SKILLS as s (s)}<option value={s}>{SKILL_NAMES[s]}</option>{/each}
        </select>
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={settings.typing.predict} /> Predict the skill from the rally
      </label>
    </div>
    <table class="list compact">
      <thead>
        <tr
          ><th></th>{#each SKILLS as s (s)}<th>{SKILL_NAMES[s]}</th>{/each}</tr
        >
      </thead>
      <tbody>
        <tr>
          <td>Type</td>
          {#each SKILLS as s, i (s)}
            <td>
              <select bind:value={settings.typing.types[i]} aria-label={`${SKILL_NAMES[s]} type`}>
                {#each TYPES as t (t)}<option value={t}>{t}</option>{/each}
              </select>
            </td>
          {/each}
        </tr>
        <tr>
          <td>Evaluation</td>
          {#each SKILLS as s, i (s)}
            <td>
              <select
                bind:value={settings.typing.evaluations[i]}
                aria-label={`${SKILL_NAMES[s]} evaluation`}
              >
                {#each EVALUATIONS as e (e)}<option value={e}>{e}</option>{/each}
              </select>
            </td>
          {/each}
        </tr>
      </tbody>
    </table>

    <h2>Rules</h2>
    <p class="sub-note">
      The competition's options. The checks and the court panel follow them in every match of the
      season.
    </p>
    <div class="pick">
      <label
        >Substitutions per set
        <select bind:value={settings.rules.substitutions}>
          {#each [6, 8, 12] as n (n)}<option value={n}>{n}</option>{/each}
        </select>
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={settings.rules.liberoMayServe} /> A libero may serve
      </label>
      <label class="inline">
        <input type="checkbox" bind:checked={settings.rules.confirmSetEnd} /> Confirm each set end before
        the console writes it
      </label>
    </div>

    <h2>Video</h2>
    <p class="sub-note">
      How codes get their place in the video. Exact: each code where its first key was typed.
      Predictive: a rally's first code where it was typed, each later touch a fixed step after the
      one before; suits coding live, behind the play.
    </p>
    <div class="pick">
      <label
        >Video times
        <select bind:value={settings.video.predictive}>
          <option value={false}>Exact</option>
          <option value={true}>Predictive</option>
        </select>
      </label>
      <label
        >Step between touches
        <select bind:value={settings.video.stepMs} disabled={!settings.video.predictive}>
          {#each [800, 1000, 1200, 1500, 2000, 2500] as ms (ms)}<option value={ms}
              >{ms / 1000} s</option
            >{/each}
        </select>
      </label>
      <label
        >Clips start before the code
        <select bind:value={settings.video.preRollMs}>
          {#each [0, 1000, 2000, 3000, 4000, 5000, 8000] as ms (ms)}<option value={ms}
              >{ms / 1000} s</option
            >{/each}
        </select>
      </label>
      <label
        >and run on after it
        <select bind:value={settings.video.postRollMs}>
          {#each [0, 1000, 2000, 3000, 4000, 5000, 8000] as ms (ms)}<option value={ms}
              >{ms / 1000} s</option
            >{/each}
        </select>
      </label>
    </div>

    <h2>Winning symbols</h2>
    <p class="sub-note">Evaluations that end the rally, for the touching team.</p>
    <table class="list compact">
      <thead><tr><th>Skill</th><th>Loses the point</th><th>Wins the point</th></tr></thead>
      <tbody>
        {#each settings.tables.winning as w (w.skill)}
          <tr>
            <td>{SKILL_NAMES[w.skill] ?? w.skill}</td>
            <td><input class="mono short" bind:value={w.lose} maxlength="4" /></td>
            <td><input class="mono short" bind:value={w.win} maxlength="4" /></td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h2>Attack combinations</h2>
    <table class="list compact">
      <thead>
        <tr
          ><th>Code</th><th>Start zone</th><th>Type</th><th>Set to</th><th>Description</th><th
          ></th></tr
        >
      </thead>
      <tbody>
        {#each settings.tables.combinations as c, i (i)}
          <tr>
            <td><input class="mono short" bind:value={c.code} maxlength="2" aria-label="Code" /></td
            >
            <td>
              <input
                type="number"
                min="1"
                max="9"
                class="short"
                bind:value={c.startZone}
                aria-label="Start zone"
              />
            </td>
            <td>
              <select bind:value={c.tempo} aria-label="Type">
                {#each TYPES as t (t)}<option value={t}>{t}</option>{/each}
              </select>
            </td>
            <td>
              <select bind:value={c.target} aria-label="Set to">
                <option value="">–</option>
                <option value="F">Front</option>
                <option value="B">Back</option>
                <option value="C">Centre</option>
                <option value="P">Pipe</option>
                <option value="S">Setter</option>
              </select>
            </td>
            <td><input bind:value={c.description} aria-label="Description" /></td>
            <td>
              <button
                type="button"
                class="icon-btn"
                aria-label={`Remove ${c.code}`}
                onclick={() => settings?.tables.combinations.splice(i, 1)}>×</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="actions">
      <button
        class="btn"
        type="button"
        onclick={() =>
          settings?.tables.combinations.push({
            code: "",
            startZone: 4,
            tempo: "H",
            target: "",
            description: "",
          })}>Add combination</button
      >
    </div>

    <h2>Setter calls</h2>
    <table class="list compact">
      <thead><tr><th>Code</th><th>Description</th><th></th></tr></thead>
      <tbody>
        {#each settings.tables.setterCalls as k, i (i)}
          <tr>
            <td><input class="mono short" bind:value={k.code} maxlength="2" aria-label="Code" /></td
            >
            <td><input bind:value={k.description} aria-label="Description" /></td>
            <td>
              <button
                type="button"
                class="icon-btn"
                aria-label={`Remove ${k.code}`}
                onclick={() => settings?.tables.setterCalls.splice(i, 1)}>×</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="actions">
      <button
        class="btn"
        type="button"
        onclick={() => settings?.tables.setterCalls.push({ code: "K", description: "" })}
        >Add setter call</button
      >
    </div>

    <h2>Keys</h2>
    <p class="sub-note">Click a key, then press the new one. Mod is Ctrl, or Cmd on a Mac.</p>
    <KeymapEditor bind:keymap={settings.keymap} />

    <h2>Keys that type</h2>
    <p class="sub-note">
      Remap a key on the command line: it types the text instead of its own character (for example ]
      types # and [ types +).
    </p>
    <KeyTextEditor bind:keyText={settings.keyText} keymap={effectiveKeymap(settings.keymap)} />

    {#if error}<p class="error">{error}</p>{/if}
    {#if note}<p class="sub-note">{note}</p>{/if}
    <div class="actions">
      <button class="btn primary" type="button" onclick={save}>Save settings</button>
    </div>
  </section>
{:else if error}
  <p class="error">{error}</p>
{/if}
